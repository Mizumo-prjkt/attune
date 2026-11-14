"""
Test Harness & Utilities for Windows Runtime, Packaging, Code Signing & Verification Pipeline.

Authoritative Specifications:
- ORIGINAL_REQUEST.md
- PROJECT.md
- TEST_INFRA.md
- SCOPE.md
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import re
import shutil
import struct
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

import yaml

# -----------------------------------------------------------------------------
# Workspace Paths & Required Target Constants
# -----------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CRATES_DIR = REPO_ROOT / "crates"
PACKAGING_DIR = REPO_ROOT / "packaging" / "windows"
TOOLS_DIR = REPO_ROOT / "tools"
WORKFLOWS_DIR = REPO_ROOT / ".github" / "workflows"

# The 5 workspace dynamic DLLs required by Attune desktop runtime
REQUIRED_DYNAMIC_DLLS = [
    "attune_engine.dll",
    "attune_watchguard.dll",
    "attune_procmgr.dll",
    "attune_music_metadata.dll",
    "attune_music_metadata_changer.dll",
]

REQUIRED_PE_BINARIES = ["attune-desktop.exe"] + REQUIRED_DYNAMIC_DLLS

# Expected upgrade flags for dynamic components in Inno Setup
REQUIRED_DLL_UPGRADE_FLAGS = {
    "replacesameversion",
    "comparetimestamp",
    "overwritereadonly",
    "restartreplace",
    "uninsremovereadonly",
    "ignoreversion",
}


# -----------------------------------------------------------------------------
# Inno Setup Script Parser (.iss)
# -----------------------------------------------------------------------------

class InnoSetupParser:
    """
    Parser and AST-like analyzer for Inno Setup (.iss) scripts.
    Handles sections ([Setup], [Files], [Tasks], [Icons], [Run], etc.),
    preprocessor directives (#define, #if, #else, #endif),
    and entry parameters with flag sets.
    """

    def __init__(self, content: str):
        self.raw_content = content
        self.sections: Dict[str, List[str]] = {}
        self.macros: Dict[str, str] = {}
        self._parse()

    @classmethod
    def parse_file(cls, path: Path | str) -> InnoSetupParser:
        target = Path(path)
        if not target.exists():
            raise FileNotFoundError(f"Inno Setup script not found at {target}")
        return cls(target.read_text(encoding="utf-8", errors="replace"))

    def _parse(self) -> None:
        current_section: Optional[str] = None
        for raw_line in self.raw_content.splitlines():
            line = raw_line.strip()
            if not line or line.startswith(";"):
                continue

            # Preprocessor #define
            if line.startswith("#define"):
                parts = line[7:].strip().split(None, 1)
                if len(parts) >= 1:
                    macro_name = parts[0]
                    macro_val = parts[1].strip().strip('"') if len(parts) > 1 else ""
                    self.macros[macro_name] = macro_val
                continue

            # Section headers [SectionName]
            section_match = re.match(r"^\[([a-zA-Z0-9_]+)\]$", line)
            if section_match:
                current_section = section_match.group(1).lower()
                if current_section not in self.sections:
                    self.sections[current_section] = []
                continue

            if current_section:
                self.sections[current_section].append(line)

    def get_macro(self, name: str) -> Optional[str]:
        return self.macros.get(name)

    def get_setup_directive(self, key: str) -> Optional[str]:
        """
        Retrieves the value of a key-value directive in the [Setup] section.
        Case-insensitive key lookup.
        """
        setup_lines = self.sections.get("setup", [])
        for line in setup_lines:
            if "=" in line:
                k, v = line.split("=", 1)
                if k.strip().lower() == key.lower():
                    return v.strip().strip('"')
        return None

    def get_files_entries(self) -> List[Dict[str, Any]]:
        """
        Parses all entries in the [Files] section into structured dictionaries.
        Each entry has: source, dest_dir, dest_name, flags (Set[str]), raw.
        """
        entries = []
        files_lines = self.sections.get("files", [])
        for line in files_lines:
            # Skip conditional preprocessor lines inside [Files]
            if line.startswith("#"):
                continue
            entry = self._parse_param_line(line)
            if "source" in entry:
                entries.append(entry)
        return entries

    def _parse_param_line(self, line: str) -> Dict[str, Any]:
        params: Dict[str, Any] = {"raw": line, "flags": set()}
        # Inno Setup parameter syntax: Key: "Value"; Key2: Value2; Flags: flag1 flag2
        pattern = re.compile(r'([a-zA-Z]+)\s*:\s*([^;]+)(?:;|$)')
        for match in pattern.finditer(line):
            key = match.group(1).lower()
            val = match.group(2).strip().strip('"')
            if key == "flags":
                # Handle macro expansion in flags: {#DllUpgradeFlags}
                expanded_flags = val
                for m_name, m_val in self.macros.items():
                    expanded_flags = expanded_flags.replace(f"{{#{m_name}}}", m_val)
                # Split whitespace-separated flags
                params["flags"] = set(f.lower() for f in expanded_flags.split())
            else:
                params[key] = val
        return params

    def find_files_by_dest_dir(self, dest_dir: str) -> List[Dict[str, Any]]:
        norm_dir = dest_dir.replace("/", "\\").lower()
        return [
            e for e in self.get_files_entries()
            if e.get("destdir", "").replace("/", "\\").lower() == norm_dir
        ]

    def find_files_by_dest_name(self, dest_name: str) -> List[Dict[str, Any]]:
        norm_name = dest_name.lower()
        results = []
        for e in self.get_files_entries():
            dname = e.get("destname", "")
            if dname:
                if dname.lower() == norm_name:
                    results.append(e)
            else:
                # Defaults to source filename
                src = Path(e.get("source", "")).name.lower()
                if src == norm_name:
                    results.append(e)
        return results

    def has_signonce(self, entry: Dict[str, Any]) -> bool:
        return "signonce" in entry.get("flags", set())


# -----------------------------------------------------------------------------
# GitHub Actions Workflow Validator (.yml / .yaml)
# -----------------------------------------------------------------------------

class WorkflowValidator:
    """
    Parser and schema validator for GitHub Actions workflow definitions.
    Inspects triggers, jobs, runners, steps, permissions, and embedded scripts.
    """

    def __init__(self, content: str):
        self.raw_content = content
        self.data: Dict[str, Any] = yaml.safe_load(content) or {}

    @classmethod
    def parse_file(cls, path: Path | str) -> WorkflowValidator:
        target = Path(path)
        if not target.exists():
            raise FileNotFoundError(f"Workflow file not found at {target}")
        return cls(target.read_text(encoding="utf-8", errors="replace"))

    def get_triggers(self) -> List[str]:
        on_clause = self.data.get("on") or self.data.get(True)  # PyYAML parses 'on' as boolean True
        if not on_clause:
            return []
        if isinstance(on_clause, str):
            return [on_clause]
        if isinstance(on_clause, list):
            return on_clause
        if isinstance(on_clause, dict):
            return list(on_clause.keys())
        return []

    def get_jobs(self) -> Dict[str, Any]:
        return self.data.get("jobs", {})

    def get_job(self, job_name: str) -> Optional[Dict[str, Any]]:
        return self.get_jobs().get(job_name)

    def get_job_runs_on(self, job_name: str) -> Optional[str]:
        job = self.get_job(job_name)
        if not job:
            return None
        return job.get("runs-on")

    def get_job_needs(self, job_name: str) -> List[str]:
        job = self.get_job(job_name)
        if not job:
            return []
        needs = job.get("needs", [])
        if isinstance(needs, str):
            return [needs]
        return list(needs)

    def get_job_steps(self, job_name: str) -> List[Dict[str, Any]]:
        job = self.get_job(job_name)
        if not job:
            return []
        return job.get("steps", [])

    def find_step(self, job_name: str, name_substr: str = "", uses_substr: str = "") -> Optional[Dict[str, Any]]:
        for step in self.get_job_steps(job_name):
            if name_substr and name_substr.lower() in step.get("name", "").lower():
                return step
            if uses_substr and uses_substr.lower() in step.get("uses", "").lower():
                return step
        return None

    def get_permissions(self) -> Dict[str, str]:
        return self.data.get("permissions", {})

    def extract_powershell_scripts(self, job_name: Optional[str] = None) -> List[str]:
        scripts = []
        jobs = [self.get_job(job_name)] if job_name else self.get_jobs().values()
        for j in jobs:
            if not j:
                continue
            for s in j.get("steps", []):
                shell = s.get("shell", "")
                if shell in ("pwsh", "powershell") or ("run" in s and not shell):
                    run_cmd = s.get("run", "")
                    if run_cmd:
                        scripts.append(run_cmd)
        return scripts


# -----------------------------------------------------------------------------
# Synthetic PE Binary Builder & Authenticode Security Directory Parser
# -----------------------------------------------------------------------------

class PEParser:
    """
    Lightweight, dependency-free Portable Executable (PE32+) parser
    capable of parsing DOS headers, PE headers, Optional Headers,
    and verifying the IMAGE_DIRECTORY_ENTRY_SECURITY (Certificate Table).
    """

    IMAGE_FILE_MACHINE_AMD64 = 0x8664
    IMAGE_NT_OPTIONAL_HDR64_MAGIC = 0x020B
    IMAGE_DIRECTORY_ENTRY_SECURITY_INDEX = 4

    @classmethod
    def parse_bytes(cls, data: bytes) -> Dict[str, Any]:
        if len(data) < 64:
            return {"is_pe": False, "error": "Buffer too short for DOS header"}

        # DOS Header check: 'MZ'
        if data[:2] != b"MZ":
            return {"is_pe": False, "error": "Missing MZ signature"}

        e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        if e_lfanew + 24 > len(data):
            return {"is_pe": False, "error": "Invalid e_lfanew offset"}

        # PE Signature: 'PE\0\0'
        if data[e_lfanew:e_lfanew + 4] != b"PE\x00\x00":
            return {"is_pe": False, "error": "Missing PE signature"}

        coff_offset = e_lfanew + 4
        machine, num_sections, _, _, _, size_of_opt_hdr, characteristics = struct.unpack_from(
            "<HHIIIHH", data, coff_offset
        )

        opt_offset = coff_offset + 20
        if size_of_opt_hdr < 2:
            return {"is_pe": True, "is_signed": False, "machine": machine}

        magic = struct.unpack_from("<H", data, opt_offset)[0]
        is_64bit = (magic == cls.IMAGE_NT_OPTIONAL_HDR64_MAGIC)

        # In PE32+ (64-bit), DataDirectory starts at opt_offset + 112 (offset 0x70)
        # In PE32 (32-bit), DataDirectory starts at opt_offset + 96 (offset 0x60)
        data_dir_base = opt_offset + (112 if is_64bit else 96)
        sec_dir_offset = data_dir_base + (cls.IMAGE_DIRECTORY_ENTRY_SECURITY_INDEX * 8)

        sec_dir_va = 0
        sec_dir_size = 0
        if sec_dir_offset + 8 <= len(data):
            sec_dir_va, sec_dir_size = struct.unpack_from("<II", data, sec_dir_offset)

        has_security_dir = (sec_dir_va > 0 and sec_dir_size > 0)
        is_signed = False
        cert_info = None

        if has_security_dir and sec_dir_va + sec_dir_size <= len(data):
            # Parse WIN_CERTIFICATE structure
            cert_raw = data[sec_dir_va:sec_dir_va + sec_dir_size]
            if len(cert_raw) >= 8:
                dw_length, w_revision, w_cert_type = struct.unpack_from("<IHH", cert_raw, 0)
                is_signed = (dw_length > 8 and w_cert_type == 0x0002)
                cert_info = {
                    "length": dw_length,
                    "revision": hex(w_revision),
                    "type": hex(w_cert_type),
                    "payload_len": len(cert_raw) - 8,
                }

        return {
            "is_pe": True,
            "machine": machine,
            "is_64bit": is_64bit,
            "num_sections": num_sections,
            "sec_dir_va": sec_dir_va,
            "sec_dir_size": sec_dir_size,
            "is_signed": is_signed,
            "cert_info": cert_info,
        }

    @classmethod
    def parse_file(cls, path: Path | str) -> Dict[str, Any]:
        target = Path(path)
        if not target.exists():
            return {"is_pe": False, "error": f"File not found: {target}"}
        return cls.parse_bytes(target.read_bytes())


class AuthenticodeHelper:
    """
    Generator and mutator for synthetic PE binaries and Authenticode structures.
    Allows testing full PE signing, certificate table placement, and anti-tamper
    hash modification without requiring native Windows tools or external compilers.
    """

    @classmethod
    def create_synthetic_pe(
        cls,
        path: Path | str,
        is_signed: bool = False,
        cert_subject: str = "CN=Attune Production Code Signing",
    ) -> Path:
        """
        Creates a structurally valid 64-bit PE binary (x64) with DOS stub,
        PE header, Optional Header (PE32+), and optional WIN_CERTIFICATE table.
        """
        out_path = Path(path)
        out_path.parent.mkdir(parents=True, exist_ok=True)

        e_lfanew = 0x80
        dos_header = bytearray(e_lfanew)
        dos_header[0:2] = b"MZ"
        struct.pack_into("<I", dos_header, 0x3C, e_lfanew)

        # PE Signature
        pe_sig = b"PE\x00\x00"

        # COFF File Header (Machine: AMD64, 1 section, opt hdr size = 240, exec image)
        coff = struct.pack("<HHIIIHH", 0x8664, 1, 0, 0, 0, 240, 0x0022)

        # PE32+ Optional Header: 240 bytes total (112 standard/windows fields + 16 DataDirectories * 8 = 128)
        opt_hdr = bytearray(240)
        struct.pack_into("<H", opt_hdr, 0, 0x020B)  # Magic = PE32+
        struct.pack_into("<I", opt_hdr, 16, 0x1000)  # AddressOfEntryPoint
        struct.pack_into("<Q", opt_hdr, 24, 0x140000000)  # ImageBase
        struct.pack_into("<I", opt_hdr, 32, 0x1000)  # SectionAlignment
        struct.pack_into("<I", opt_hdr, 36, 0x200)   # FileAlignment
        struct.pack_into("<I", opt_hdr, 108, 16)     # NumberOfRvaAndSizes

        # Section Header: .text (40 bytes)
        sec_name = b".text\x00\x00\x00"
        virt_size = 0x200
        virt_addr = 0x1000
        raw_size = 0x200
        raw_ptr = 0x200
        sec_hdr = struct.pack(
            "<8sIIIIIIHHI",
            sec_name,
            virt_size,
            virt_addr,
            raw_size,
            raw_ptr,
            0,
            0,
            0,
            0,
            0x60000020,  # Code, Execute, Read
        )

        # Build initial binary without certificate
        pre_headers = dos_header + pe_sig + coff
        # Pad up to raw_ptr
        file_bytes = bytearray(raw_ptr + raw_size)
        file_bytes[0:len(pre_headers)] = pre_headers

        opt_offset = len(pre_headers)
        file_bytes[opt_offset:opt_offset + len(opt_hdr)] = opt_hdr
        sec_offset = opt_offset + len(opt_hdr)
        file_bytes[sec_offset:sec_offset + len(sec_hdr)] = sec_hdr

        # Put dummy code in .text
        file_bytes[raw_ptr:raw_ptr + 16] = b"\x48\x31\xC0\xC3\x90\x90\x90\x90\x90\x90\x90\x90\x90\x90\x90\x90"

        if is_signed:
            # Build WIN_CERTIFICATE table
            cert_payload = cert_subject.encode("utf-8")
            cert_len = 8 + len(cert_payload)
            # Must be 8-byte aligned
            aligned_cert_len = (cert_len + 7) & ~7
            win_cert = bytearray(aligned_cert_len)
            struct.pack_into("<IHH", win_cert, 0, cert_len, 0x0200, 0x0002)
            win_cert[8:8 + len(cert_payload)] = cert_payload

            # Append certificate table at end of file
            cert_offset = len(file_bytes)
            file_bytes.extend(win_cert)

            # Update Security Directory in Optional Header
            sec_dir_offset = opt_offset + 112 + (4 * 8)
            struct.pack_into("<II", file_bytes, sec_dir_offset, cert_offset, aligned_cert_len)

        out_path.write_bytes(file_bytes)
        return out_path

    @classmethod
    def append_authenticode_signature(
        cls,
        path: Path | str,
        cert_subject: str = "CN=Attune SignTool CounterSignature",
    ) -> Path:
        """
        Signs an existing PE binary by appending a WIN_CERTIFICATE table and
        updating the Security Directory entry in the Optional Header.
        """
        target = Path(path)
        data = bytearray(target.read_bytes())
        parsed = PEParser.parse_bytes(data)
        if not parsed["is_pe"]:
            raise ValueError(f"Cannot sign non-PE file: {target}")

        e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        coff_offset = e_lfanew + 4
        opt_offset = coff_offset + 20
        sec_dir_offset = opt_offset + 112 + (4 * 8)

        cert_payload = cert_subject.encode("utf-8")
        cert_len = 8 + len(cert_payload)
        aligned_cert_len = (cert_len + 7) & ~7
        win_cert = bytearray(aligned_cert_len)
        struct.pack_into("<IHH", win_cert, 0, cert_len, 0x0200, 0x0002)
        win_cert[8:8 + len(cert_payload)] = cert_payload

        cert_offset = len(data)
        data.extend(win_cert)
        struct.pack_into("<II", data, sec_dir_offset, cert_offset, aligned_cert_len)

        target.write_bytes(data)
        return target


# -----------------------------------------------------------------------------
# Windows Staging & Pipeline Simulation Fixture
# -----------------------------------------------------------------------------

class WindowsStagingFixture:
    """
    Manages an isolated staging environment simulating Windows packaging,
    dynamic component layout, certificate decoding, and integrity manifests.
    """

    def __init__(self):
        self.tmp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp_dir.name)
        self.dist_dir = self.root / "dist" / "windows"
        self.dist_lib_dir = self.dist_dir / "lib"
        self.target_release_dir = self.root / "target" / "release"

        self.dist_dir.mkdir(parents=True, exist_ok=True)
        self.dist_lib_dir.mkdir(parents=True, exist_ok=True)
        self.target_release_dir.mkdir(parents=True, exist_ok=True)

    def cleanup(self) -> None:
        self.tmp_dir.cleanup()

    def stage_binaries(self, signed: bool = False) -> Dict[str, Path]:
        """
        Populates `{app}` (dist_dir), `{app}/lib` (dist_lib_dir), and `target/release`
        with the required desktop executable and 5 workspace dynamic DLLs.
        """
        paths = {}
        # Desktop executable
        desktop_path = AuthenticodeHelper.create_synthetic_pe(
            self.dist_dir / "attune-desktop.exe", is_signed=signed
        )
        shutil.copy2(desktop_path, self.target_release_dir / "attune-desktop.exe")
        paths["desktop"] = desktop_path

        # 5 Dynamic DLLs
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            dll_in_lib = AuthenticodeHelper.create_synthetic_pe(
                self.dist_lib_dir / dll_name, is_signed=signed
            )
            shutil.copy2(dll_in_lib, self.dist_dir / dll_name)
            shutil.copy2(dll_in_lib, self.target_release_dir / dll_name)
            paths[dll_name] = dll_in_lib

        return paths

    def compute_file_hashes(self, path: Path) -> Dict[str, str]:
        data = path.read_bytes()
        sha256_hash = hashlib.sha256(data).hexdigest()
        blake3_hash = None
        try:
            import blake3
            blake3_hash = blake3.blake3(data).hexdigest()
        except ImportError:
            blake3_hash = sha256_hash
        return {
            "size_bytes": len(data),
            "sha256": sha256_hash,
            "blake3": blake3_hash,
        }

    def generate_manifest(self, version: str = "0.1.0") -> Dict[str, Any]:
        """
        Generates attune_integrity.json in both dist_dir and dist_lib_dir.
        Uses exact same schema and hash priority as `tools/builder.py`.
        """
        components = {}
        # Search dist_dir and dist_lib_dir
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            f = self.dist_lib_dir / dll_name
            if f.exists():
                components[dll_name] = self.compute_file_hashes(f)

        manifest = {
            "version": version,
            "dynamic_version": f"v{version}-test",
            "build_number": 42,
            "generated_at": "2026-11-14T00:00:00Z",
            "components": components,
        }

        manifest_json = json.dumps(manifest, indent=2)
        (self.dist_dir / "attune_integrity.json").write_text(manifest_json, encoding="utf-8")
        (self.dist_lib_dir / "attune_integrity.json").write_text(manifest_json, encoding="utf-8")
        return manifest

    def verify_component_integrity(self, file_path: Path, expected_sha256: str) -> Tuple[bool, str]:
        """
        Simulates `dynamic_components::verify_component_integrity` in `attune-desktop`.
        Returns (is_valid, diagnostic_message).
        """
        if not file_path.exists():
            return False, f"Component not found at {file_path}"
        data = file_path.read_bytes()
        computed = hashlib.sha256(data).hexdigest()
        if not computed.lower() == expected_sha256.lower().strip():
            return False, (
                f"CRITICAL: SHA-256 signature mismatch on '{file_path.name}'! "
                f"Expected {expected_sha256}, computed {computed}. Load rejected."
            )
        return True, "Integrity verified successfully"


# -----------------------------------------------------------------------------
# Rust Dynamic Loader & Win32 Contract Emulation
# -----------------------------------------------------------------------------

class RustLoaderContract:
    """
    Emulates and validates the UTF-16 wide string conversions, null termination,
    and DLL directory search semantics required by Win32 `SetDllDirectoryW`
    and `LoadLibraryW` contracts in `crates/attune-desktop`.
    """

    @classmethod
    def encode_wide(cls, s: str) -> List[int]:
        """Replicates Rust `OsStrExt::encode_wide` with trailing null terminator."""
        u16_list = list(s.encode("utf-16-le"))
        # Group into 16-bit words
        words = [u16_list[i] | (u16_list[i + 1] << 8) for i in range(0, len(u16_list), 2)]
        words.append(0)  # Null terminator
        return words

    @classmethod
    def decode_wide(cls, words: List[int]) -> str:
        """Converts wide word list back to Python string, discarding null terminator."""
        clean_words = [w for w in words if w != 0]
        raw_bytes = bytearray()
        for w in clean_words:
            raw_bytes.append(w & 0xFF)
            raw_bytes.append((w >> 8) & 0xFF)
        return raw_bytes.decode("utf-16-le")

    @classmethod
    def validate_null_termination(cls, words: List[int]) -> bool:
        if not words:
            return False
        return words[-1] == 0 and 0 not in words[:-1]

    @classmethod
    def simulate_search_order(
        cls,
        dll_name: str,
        set_dll_directory: Optional[Path],
        app_dir: Path,
        system_dirs: List[Path],
    ) -> List[Path]:
        """
        Simulates Windows dynamic linker search order.
        When SetDllDirectoryW is called, the specified directory is searched
        immediately after the application directory and before system directories.
        """
        order = [app_dir / dll_name]
        if set_dll_directory:
            order.append(set_dll_directory / dll_name)
        for sdir in system_dirs:
            order.append(sdir / dll_name)
        return order


# -----------------------------------------------------------------------------
# Base64 Certificate Helper
# -----------------------------------------------------------------------------

class Base64CertHelper:
    """
    Generates synthetic PKCS#12 (.pfx) payloads and tests base64 decoding,
    temporary file staging, and password passing.
    """

    @classmethod
    def generate_dummy_pfx(cls, password: str = "TestSecret123!") -> bytes:
        """
        Builds a synthetic binary PKCS#12 structure with DER-like header
        and embedded password metadata for testing decoding pipelines.
        """
        header = b"\x30\x82\x04\x00\x02\x01\x03"  # Standard PKCS#12 DER header prefix
        payload = f"PFX_KEY_DATA_PASSWORD={password}".encode("utf-8")
        padding = b"\x00" * (128 - len(payload))
        return header + payload + padding

    @classmethod
    def encode_base64(cls, data: bytes) -> str:
        return base64.b64encode(data).decode("ascii")

    @classmethod
    def decode_base64(cls, b64_str: str) -> bytes:
        return base64.b64decode(b64_str)

    @classmethod
    def resolve_cert_to_file(cls, b64_str: str, out_path: Path) -> Path:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        raw_bytes = cls.decode_base64(b64_str)
        out_path.write_bytes(raw_bytes)
        return out_path


# -----------------------------------------------------------------------------
# Reference Specification Templates (Authoritative Oracles for Contracts)
# -----------------------------------------------------------------------------

REFERENCE_ISS_SETUP_SIGNTOOL = """
#define SignToolName "signtool"

[Setup]
AppName=Attune
AppVersion=0.1.0
SignTool={#SignToolName}
SignedUninstaller=yes
OutputDir=dist\\windows
"""

REFERENCE_VERIFY_SIGNATURES_WORKFLOW = """
name: Verify Authenticode Signatures
on:
  workflow_dispatch:
    inputs:
      run_id:
        description: 'GitHub Actions Run ID'
        required: false
      build_type:
        description: 'Build Type'
        required: false
        default: 'Release'
  workflow_call:
    inputs:
      run_id:
        required: false
        type: string
      build_type:
        required: false
        type: string
        default: 'Release'
    secrets:
      WINDOWS_CERT_BASE64:
        required: false
      WINDOWS_CERT_PASS:
        required: false
  workflow_run:
    workflows: ["Build and Deploy"]
    types: [completed]

permissions:
  contents: read
  actions: read

jobs:
  verify_authenticode:
    name: Verify Authenticode Signatures (Windows)
    runs-on: windows-2022
    steps:
      - name: Checkout Repository
        uses: actions/checkout@v4

      - name: Setup Windows Trust Anchor
        shell: pwsh
        env:
          WINDOWS_CERT_BASE64: ${{ secrets.WINDOWS_CERT_BASE64 }}
          WINDOWS_CERT_PASS: ${{ secrets.WINDOWS_CERT_PASS }}
        run: |
          if ($env:WINDOWS_CERT_BASE64) {
            $certBytes = [System.Convert]::FromBase64String($env:WINDOWS_CERT_BASE64)
            $cert = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certBytes, $env:WINDOWS_CERT_PASS)
            $store = [System.Security.Cryptography.X509Certificates.X509Store]::new("Root", "LocalMachine")
            $store.Open("ReadWrite")
            $store.Add($cert)
            $store.Close()
          }

      - name: Verify Embedded Authenticode & Dual Signatures
        shell: pwsh
        run: |
          $targets = @("attune-desktop.exe", "attune_engine.dll", "attune_watchguard.dll", "attune_procmgr.dll", "attune_music_metadata.dll", "attune_music_metadata_changer.dll")
          foreach ($t in $targets) {
            $sig = Get-AuthenticodeSignature $t
            if ($sig.Status -ne 'Valid') {
              throw "Signature verification failed on $t : $($sig.StatusMessage)"
            }
            if (-not $sig.TimeStamperCertificate) {
              throw "Missing RFC 3161 timestamp on $t"
            }
          }
"""
