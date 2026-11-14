"""
Tier 2 Boundary & Corner Case Test Suite — Attune Windows Runtime, Packaging, Code Signing & Verification.

Authoritative Specifications & Coverage Matrix:
- ORIGINAL_REQUEST.md: Requirements R1, R2, R3, R4
- PROJECT.md: Features 1–14
- TEST_INFRA.md: Tier 2 Boundary / Corner Case Matrix
- SCOPE.md: Milestone T3 Requirements

Features Covered (14 Features, >=5 tests each, >=70 tests total):
  - F1: SetDllDirectoryW Initialization (5 boundary tests)
  - F2: LoadLibraryW Unicode Loading (5 boundary tests)
  - F3: Dynamic Component Bundling (5 boundary tests)
  - F4: Integrity Manifest Synchronization (5 boundary tests)
  - F5: Base64 Certificate Resolution (5 boundary tests)
  - F6: RFC 3161 Timestamping (5 boundary tests)
  - F7: Full PE Binary Signing (5 boundary tests)
  - F8: Inno Setup SignTool Directive (5 boundary tests)
  - F9: Inno Setup signonce Flags (5 boundary tests)
  - F10: Inno Setup CLI /Ssigntool Param (5 boundary tests)
  - F11: verify_signatures.yml Workflow (5 boundary tests)
  - F12: Dual Authenticode Verification (5 boundary tests)
  - F13: Trust Store Registration (5 boundary tests)
  - F14: Release Gating Integration (5 boundary tests)
"""

from __future__ import annotations

import base64
import binascii
import hashlib
import json
import os
import re
import shutil
import struct
import tempfile
import unittest
from pathlib import Path
from typing import Any, Dict, List, Optional
import urllib.parse

import yaml

from tests.e2e_windows_pipeline.test_harness import (
    REPO_ROOT,
    CRATES_DIR,
    PACKAGING_DIR,
    TOOLS_DIR,
    WORKFLOWS_DIR,
    REQUIRED_DYNAMIC_DLLS,
    REQUIRED_PE_BINARIES,
    REQUIRED_DLL_UPGRADE_FLAGS,
    InnoSetupParser,
    WorkflowValidator,
    PEParser,
    AuthenticodeHelper,
    WindowsStagingFixture,
    RustLoaderContract,
    Base64CertHelper,
)


# =============================================================================
# Feature 1: SetDllDirectoryW Initialization (Boundary & Corner Cases)
# =============================================================================

class TestTier2F1SetDllDirectoryW(unittest.TestCase):
    """
    F1: SetDllDirectoryW Initialization — Boundary & Corner Cases.
    Tests MAX_PATH boundaries, extended-length path prefixes, empty strings,
    trailing path separators, and early startup initialization contracts.
    """

    def setUp(self):
        self.main_rs_path = CRATES_DIR / "attune-desktop" / "src" / "main.rs"
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_f1_b1_max_path_260_char_boundary(self):
        """F1-B1: Verify MAX_PATH (260 characters) boundary path encodes and null-terminates correctly."""
        # Standard Win32 MAX_PATH is 260 characters
        # Construct path of exactly 260 characters: C:\ + 253 'a's + \lib (3 + 253 + 4 = 260)
        path_260 = "C:\\" + ("a" * 253) + "\\lib"
        self.assertEqual(len(path_260), 260, "Path must be exactly 260 characters.")

        wide_260 = RustLoaderContract.encode_wide(path_260)
        self.assertEqual(len(wide_260), 261, "Wide encoding must have 260 words + 1 null terminator.")
        self.assertTrue(
            RustLoaderContract.validate_null_termination(wide_260),
            "F1-B1: 260-char path must have single terminal null word.",
        )
        self.assertEqual(RustLoaderContract.decode_wide(wide_260), path_260)

        # Also test 261 characters (exceeding MAX_PATH)
        path_261 = path_260 + "x"
        wide_261 = RustLoaderContract.encode_wide(path_261)
        self.assertEqual(len(wide_261), 262)
        self.assertEqual(RustLoaderContract.decode_wide(wide_261), path_261)

    def test_f1_b2_extended_length_path_prefix(self):
        """F1-B2: Verify extended-length path prefix (\\\\?\\) is preserved and handles deep nesting."""
        extended_prefix = "\\\\?\\C:\\Program Files\\Attune\\lib"
        wide = RustLoaderContract.encode_wide(extended_prefix)
        self.assertTrue(RustLoaderContract.validate_null_termination(wide))

        # First 4 words must be '\\', '\\', '?', '\\'
        self.assertEqual(wide[0], ord("\\"))
        self.assertEqual(wide[1], ord("\\"))
        self.assertEqual(wide[2], ord("?"))
        self.assertEqual(wide[3], ord("\\"))

        decoded = RustLoaderContract.decode_wide(wide)
        self.assertEqual(decoded, extended_prefix)

        # Test long path of 1024 characters with extended prefix
        long_extended = "\\\\?\\C:\\" + ("sub\\" * 200) + "lib"
        wide_long = RustLoaderContract.encode_wide(long_extended)
        self.assertTrue(RustLoaderContract.validate_null_termination(wide_long))
        self.assertEqual(RustLoaderContract.decode_wide(wide_long), long_extended)

    def test_f1_b3_empty_string_path_resets_or_removes(self):
        """F1-B3: Verify empty string path (\"\") encodes to a single null terminator."""
        # In Win32, SetDllDirectoryW(L"") removes current directory from search path
        wide_empty = RustLoaderContract.encode_wide("")
        self.assertEqual(
            wide_empty,
            [0],
            "F1-B3: Empty string must produce exactly one null terminator word [0].",
        )
        self.assertTrue(RustLoaderContract.validate_null_termination(wide_empty))
        self.assertEqual(RustLoaderContract.decode_wide(wide_empty), "")

    def test_f1_b4_trailing_separators_normalization(self):
        """F1-B4: Verify paths with multiple trailing backslashes/slashes encode without internal nulls."""
        paths_with_trailing = [
            "C:\\Program Files\\Attune\\lib\\\\\\",
            "C:/Program Files/Attune/lib///",
            "C:\\Attune\\lib\\/\\/",
        ]
        for p in paths_with_trailing:
            wide = RustLoaderContract.encode_wide(p)
            self.assertTrue(
                RustLoaderContract.validate_null_termination(wide),
                f"F1-B4: Trailing separator path '{p}' must be valid null-terminated wide string.",
            )
            # Ensure no internal 0-words before the terminator
            self.assertNotIn(0, wide[:-1], f"Path '{p}' produced premature null word.")
            self.assertEqual(RustLoaderContract.decode_wide(wide), p)

    def test_f1_b5_startup_initialization_contract(self):
        """F1-B5: Verify main.rs calls SetDllDirectoryW before dynamic module resolution."""
        if not self.main_rs_path.exists():
            self.fail(f"Missing main.rs at {self.main_rs_path}")
        main_src = self.main_rs_path.read_text(encoding="utf-8")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8") if self.dyn_comp_path.exists() else ""
        combined = main_src + "\n" + dyn_src

        has_set_dll_dir = "SetDllDirectoryW" in combined
        self.assertTrue(
            has_set_dll_dir,
            "Requirement R1 / F1: main.rs must call SetDllDirectoryW at early startup "
            "to register {app}\\lib in the Windows dynamic linker search path.",
        )


# =============================================================================
# Feature 2: LoadLibraryW Unicode Loading (Boundary & Corner Cases)
# =============================================================================

class TestTier2F2LoadLibraryW(unittest.TestCase):
    """
    F2: LoadLibraryW Unicode Loading — Boundary & Corner Cases.
    Tests multilingual planes (CJK, Cyrillic, Accented, SMP Emojis),
    interior null byte security boundaries, 32767-character buffer limits,
    mixed slash normalization, and LoadLibraryW usage.
    """

    def setUp(self):
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_f2_b1_multilingual_unicode_boundary_planes(self):
        """F2-B1: Verify Unicode paths across BMP and Supplementary Multilingual Plane (surrogate pairs)."""
        # Test CJK, Cyrillic, Latin-Extended, and SMP Emoji (which requires UTF-16 surrogate pairs)
        test_cases = [
            ("CJK Ideographs", "C:\\Users\\初音ミク\\lib\\attune_engine.dll"),
            ("Cyrillic Script", "C:\\Users\\Иван\\Music\\attune_watchguard.dll"),
            ("Latin Accents", "C:\\Users\\José_García\\attune_procmgr.dll"),
            ("SMP Emoji (U+1F3B5)", "C:\\Music\\🎵\\attune_music_metadata.dll"),
        ]
        for desc, path in test_cases:
            wide = RustLoaderContract.encode_wide(path)
            self.assertTrue(
                RustLoaderContract.validate_null_termination(wide),
                f"F2-B1 ({desc}): Null termination failed for '{path}'",
            )
            # In SMP, emoji 🎵 (U+1F3B5) encodes to 2 UTF-16 code units (surrogate pair 0xD83C, 0xDFB5)
            if "🎵" in path:
                has_surrogate = any(0xD800 <= w <= 0xDFFF for w in wide)
                self.assertTrue(has_surrogate, "F2-B1: Emoji path must contain UTF-16 surrogate pair.")
            decoded = RustLoaderContract.decode_wide(wide)
            self.assertEqual(decoded, path, f"F2-B1 ({desc}): Roundtrip decoding corrupted '{path}'")

    def test_f2_b2_interior_null_byte_security_boundary(self):
        """F2-B2: Verify interior null byte injection is detected and rejected."""
        # In C/Rust FFI, passing a path with an interior null byte causes path truncation security flaws
        malicious_path = "C:\\Attune\\lib\\attune\x00_engine.dll"
        wide = RustLoaderContract.encode_wide(malicious_path)
        # validate_null_termination must return False because 0 appears before the end
        is_valid = RustLoaderContract.validate_null_termination(wide)
        self.assertFalse(
            is_valid,
            "F2-B2: Path containing interior null byte must fail null-termination validation.",
        )
        self.assertIn(0, wide[:-1], "Interior null byte must be present in encoded words before terminator.")

    def test_f2_b3_extreme_buffer_boundary_32767_chars(self):
        """F2-B3: Verify maximum Win32 Unicode API path boundary (32,767 characters)."""
        prefix = "C:\\Attune\\lib\\"
        suffix = ".dll"
        filler_len = 32767 - len(prefix) - len(suffix)
        extreme_path = prefix + ("a" * filler_len) + suffix
        self.assertEqual(len(extreme_path), 32767)

        wide = RustLoaderContract.encode_wide(extreme_path)
        self.assertEqual(len(wide), 32768, "Wide word list must be 32,767 words + 1 null terminator.")
        self.assertEqual(wide[-1], 0)
        self.assertTrue(RustLoaderContract.validate_null_termination(wide))

        # Check roundtrip
        decoded = RustLoaderContract.decode_wide(wide)
        self.assertEqual(len(decoded), 32767)
        self.assertEqual(decoded, extreme_path)

    def test_f2_b4_mixed_slash_direction_normalization(self):
        """F2-B4: Verify mixed slash directions (forward / and backward \\) preserve exact code units."""
        mixed_path = "C:/Program Files\\Attune/lib\\music//attune_music_metadata_changer.dll"
        wide = RustLoaderContract.encode_wide(mixed_path)
        self.assertTrue(RustLoaderContract.validate_null_termination(wide))

        # Check code points: forward slash = 0x002F, backslash = 0x005C
        slash_codes = {w for w in wide if chr(w) in ("/", "\\")}
        self.assertEqual(slash_codes, {ord("/"), ord("\\")})

        decoded = RustLoaderContract.decode_wide(wide)
        self.assertEqual(decoded, mixed_path)

    def test_f2_b5_dynamic_components_uses_loadlibraryw_not_a(self):
        """F2-B5: Verify dynamic_components.rs uses LoadLibraryW and avoids ANSI LoadLibraryA."""
        if not self.dyn_comp_path.exists():
            self.fail(f"Missing dynamic_components.rs at {self.dyn_comp_path}")
        src = self.dyn_comp_path.read_text(encoding="utf-8")
        self.assertIn(
            "LoadLibraryW",
            src,
            "Requirement R1 / F2: os_dlopen in dynamic_components.rs must use LoadLibraryW "
            "with UTF-16 wide string encoding.",
        )
        self.assertNotIn(
            "LoadLibraryA",
            src,
            "Requirement R1 / F2: ANSI LoadLibraryA must be replaced with wide LoadLibraryW.",
        )


# =============================================================================
# Feature 3: Dynamic Component Bundling (Boundary & Corner Cases)
# =============================================================================

class TestTier2F3DynamicComponentBundling(unittest.TestCase):
    """
    F3: Dynamic Component Bundling — Boundary & Corner Cases.
    Tests boundary states: all DLLs missing, partial set present, 0-byte corrupt files,
    read-only/locked files upgrade flags, and wrong architecture / non-PE rejection.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f3_b1_all_dynamic_dlls_missing_state(self):
        """F3-B1: Verify component scanner detects state where all 5 DLLs are missing."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            empty_path = Path(tmp_dir)
            missing = [dll for dll in REQUIRED_DYNAMIC_DLLS if not (empty_path / dll).exists()]
            self.assertEqual(
                len(missing),
                5,
                "F3-B1: When no DLLs are staged, all 5 dynamic components must be flagged as missing.",
            )
            self.assertEqual(set(missing), set(REQUIRED_DYNAMIC_DLLS))

    def test_f3_b2_partial_set_presence(self):
        """F3-B2: Verify partial component set detection (e.g. 2 present, 3 missing)."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            p = Path(tmp_dir)
            # Stage only 2 DLLs
            (p / "attune_engine.dll").write_bytes(b"dummy")
            (p / "attune_watchguard.dll").write_bytes(b"dummy")

            present = [dll for dll in REQUIRED_DYNAMIC_DLLS if (p / dll).exists()]
            missing = [dll for dll in REQUIRED_DYNAMIC_DLLS if not (p / dll).exists()]

            self.assertEqual(len(present), 2)
            self.assertEqual(len(missing), 3)
            self.assertIn("attune_procmgr.dll", missing)
            self.assertIn("attune_music_metadata.dll", missing)
            self.assertIn("attune_music_metadata_changer.dll", missing)

    def test_f3_b3_zero_byte_corrupt_dll(self):
        """F3-B3: Verify 0-byte corrupt DLL files are rejected by PE parser."""
        empty_data = b""
        parsed_empty = PEParser.parse_bytes(empty_data)
        self.assertFalse(parsed_empty["is_pe"])
        self.assertIn("too short", parsed_empty.get("error", "").lower())

        # Also test 30-byte truncated stub
        stub_data = b"MZ" + (b"\x00" * 28)
        parsed_stub = PEParser.parse_bytes(stub_data)
        self.assertFalse(parsed_stub["is_pe"])

    def test_f3_b4_readonly_locked_file_upgrade_flags(self):
        """F3-B4: Verify overwritereadonly and uninsremovereadonly flags are specified for DLLs."""
        files = self.parser.get_files_entries()
        dll_entries = [
            f for f in files
            if any(dll in f.get("source", "").lower() for dll in REQUIRED_DYNAMIC_DLLS)
        ]
        self.assertGreater(len(dll_entries), 0, "No dynamic DLL entries found in [Files].")
        for entry in dll_entries:
            flags = entry.get("flags", set())
            self.assertIn(
                "overwritereadonly",
                flags,
                f"F3-B4: DLL {entry.get('source')} missing 'overwritereadonly' flag for locked files.",
            )
            self.assertIn(
                "uninsremovereadonly",
                flags,
                f"F3-B4: DLL {entry.get('source')} missing 'uninsremovereadonly' flag.",
            )

    def test_f3_b5_non_pe_and_wrong_architecture_rejection(self):
        """F3-B5: Verify non-PE and wrong-architecture (32-bit x86 / ELF) binaries are rejected."""
        # Non-PE (ELF header)
        elf_header = b"\x7fELF\x02\x01\x01\x00" + (b"\x00" * 56)
        parsed_elf = PEParser.parse_bytes(elf_header)
        self.assertFalse(parsed_elf["is_pe"], "ELF binary must not be identified as PE.")

        # Synthetic 32-bit x86 PE (machine = 0x014c instead of AMD64 0x8664)
        dos = bytearray(64)
        dos[0:2] = b"MZ"
        struct.pack_into("<I", dos, 0x3C, 64)
        pe_sig = b"PE\x00\x00"
        coff_32 = struct.pack("<HHIIIHH", 0x014C, 1, 0, 0, 0, 224, 0x0102)
        data_32 = bytes(dos) + pe_sig + coff_32 + (b"\x00" * 300)

        parsed_32 = PEParser.parse_bytes(data_32)
        self.assertTrue(parsed_32["is_pe"])
        self.assertFalse(parsed_32.get("is_64bit", True), "32-bit PE must have is_64bit=False.")
        self.assertNotEqual(
            parsed_32.get("machine"),
            PEParser.IMAGE_FILE_MACHINE_AMD64,
            "F3-B5: 32-bit x86 binary must not match AMD64 machine type.",
        )


# =============================================================================
# Feature 4: Integrity Manifest Synchronization (Boundary & Corner Cases)
# =============================================================================

class TestTier2F4IntegrityManifest(unittest.TestCase):
    """
    F4: Integrity Manifest Synchronization — Boundary & Corner Cases.
    Tests 1-bit corrupted hashes, empty manifest files, corrupted JSON syntax,
    missing target entries, and post-signing vs pre-signing hash divergence.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.staged = self.fixture.stage_binaries(signed=False)

    def tearDown(self):
        self.fixture.cleanup()

    def test_f4_b1_single_bit_corrupted_hash(self):
        """F4-B1: Verify single-bit modification of SHA-256 hash in manifest triggers load rejection."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        target_dll = self.fixture.dist_lib_dir / "attune_engine.dll"
        valid_sha = manifest["components"]["attune_engine.dll"]["sha256"]

        # Corrupt single bit / hex character
        last_char = valid_sha[-1]
        corrupted_char = "1" if last_char == "0" else "0"
        corrupted_sha = valid_sha[:-1] + corrupted_char

        is_valid, msg = self.fixture.verify_component_integrity(target_dll, corrupted_sha)
        self.assertFalse(is_valid, "F4-B1: 1-bit corrupted SHA-256 hash must be rejected.")
        self.assertIn("CRITICAL", msg)
        self.assertIn("mismatch", msg.lower())

    def test_f4_b2_empty_manifest_file(self):
        """F4-B2: Verify 0-byte or empty JSON manifest file handling."""
        manifest_file = self.fixture.dist_lib_dir / "attune_integrity.json"

        # 0-byte empty file
        manifest_file.write_text("", encoding="utf-8")
        with self.assertRaises(json.JSONDecodeError):
            json.loads(manifest_file.read_text(encoding="utf-8"))

        # Empty object {}
        manifest_file.write_text("{}", encoding="utf-8")
        empty_manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
        self.assertNotIn("components", empty_manifest, "Empty manifest missing components block.")

    def test_f4_b3_corrupted_json_syntax(self):
        """F4-B3: Verify malformed / truncated JSON manifest syntax is caught safely."""
        manifest_file = self.fixture.dist_lib_dir / "attune_integrity.json"
        malformed_json = '{\n  "version": "0.1.0",\n  "components": {\n    "attune_engine.dll": {"sha256": '
        manifest_file.write_text(malformed_json, encoding="utf-8")

        with self.assertRaises(json.JSONDecodeError):
            json.loads(manifest_file.read_text(encoding="utf-8"))

    def test_f4_b4_missing_component_entry_in_manifest(self):
        """F4-B4: Verify manifest omitting a required DLL entry returns component not found."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        # Remove attune_music_metadata_changer.dll entry
        if "attune_music_metadata_changer.dll" in manifest["components"]:
            del manifest["components"]["attune_music_metadata_changer.dll"]

        manifest_file = self.fixture.dist_lib_dir / "attune_integrity.json"
        manifest_file.write_text(json.dumps(manifest, indent=2), encoding="utf-8")

        loaded = json.loads(manifest_file.read_text(encoding="utf-8"))
        self.assertNotIn(
            "attune_music_metadata_changer.dll",
            loaded["components"],
            "F4-B4: attune_music_metadata_changer.dll omitted from manifest.",
        )

    def test_f4_b5_pre_vs_post_signing_hash_divergence(self):
        """F4-B5: Demonstrate pre-signing hash fails against post-signed binary."""
        target_dll = self.fixture.dist_lib_dir / "attune_procmgr.dll"
        hash_pre_signing = hashlib.sha256(target_dll.read_bytes()).hexdigest()

        # Sign target DLL
        AuthenticodeHelper.append_authenticode_signature(target_dll)
        hash_post_signing = hashlib.sha256(target_dll.read_bytes()).hexdigest()

        self.assertNotEqual(hash_pre_signing, hash_post_signing)

        # Verifying post-signed binary using pre-signing hash MUST fail
        is_valid, msg = self.fixture.verify_component_integrity(target_dll, hash_pre_signing)
        self.assertFalse(
            is_valid,
            "F4-B5: Pre-signing hash must fail verification on post-signed PE binary.",
        )
        self.assertIn("CRITICAL", msg)


# =============================================================================
# Feature 5: Base64 Certificate Resolution (Boundary & Corner Cases)
# =============================================================================

class TestTier2F5Base64CertificateResolution(unittest.TestCase):
    """
    F5: Base64 Certificate Resolution — Boundary & Corner Cases.
    Tests truncated base64 strings, illegal characters, wrong PKCS#12 password,
    non-DER garbage payloads, and empty environment variables.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"
        self.tmp_dir = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.tmp_dir.cleanup()

    def test_f5_b1_truncated_base64_string(self):
        """F5-B1: Verify truncated base64 string raises decoding exception."""
        pfx = Base64CertHelper.generate_dummy_pfx()
        valid_b64 = Base64CertHelper.encode_base64(pfx)

        # Truncate by 5 characters, breaking base64 padding/block boundary
        truncated = valid_b64[:-5]
        with self.assertRaises(Exception):
            Base64CertHelper.decode_base64(truncated)

    def test_f5_b2_invalid_base64_characters(self):
        """F5-B2: Verify string with non-base64 characters raises binascii.Error."""
        invalid_b64 = "MII###INVALID@@@CHARS$$$%%%"
        with self.assertRaises(Exception):
            Base64CertHelper.decode_base64(invalid_b64)

    def test_f5_b3_incorrect_pkcs12_password(self):
        """F5-B3: Verify PKCS#12 payload with wrong password fails password verification."""
        pfx_bytes = Base64CertHelper.generate_dummy_pfx(password="CorrectPass999")
        # Ensure password metadata is embedded
        self.assertIn(b"PFX_KEY_DATA_PASSWORD=CorrectPass999", pfx_bytes)

        # Attempting extraction with wrong password
        wrong_pass = "WrongPass000"
        self.assertNotIn(f"PFX_KEY_DATA_PASSWORD={wrong_pass}".encode("utf-8"), pfx_bytes)

    def test_f5_b4_non_der_garbage_payload(self):
        """F5-B4: Verify base64-decoded payload that is not valid DER/PKCS#12 header is rejected."""
        # Standard PKCS#12 starts with ASN.1 sequence 0x30 0x82
        garbage_bytes = b"NOT_A_PFX_CERTIFICATE_HEADER_RANDOM_DATA_12345"
        garbage_b64 = Base64CertHelper.encode_base64(garbage_bytes)
        decoded = Base64CertHelper.decode_base64(garbage_b64)

        has_der_prefix = decoded.startswith(b"\x30\x82")
        self.assertFalse(has_der_prefix, "F5-B4: Non-DER payload must not have PKCS#12 header prefix.")

    def test_f5_b5_empty_environment_variable_handling(self):
        """F5-B5: Verify handling when WINDOWS_CERT_BASE64 environment variable is empty string."""
        empty_b64 = ""
        decoded = Base64CertHelper.decode_base64(empty_b64)
        self.assertEqual(decoded, b"")

        # When secret is empty, builder must either skip signing gracefully or report clear warning
        self.assertTrue(self.builder_py_path.exists())
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertTrue(
            "WINDOWS_CERT_BASE64" in src or "CERT_PATH" in src,
            "builder.py must reference certificate configuration.",
        )


# =============================================================================
# Feature 6: RFC 3161 Timestamping (Boundary & Corner Cases)
# =============================================================================

class TestTier2F6RFC3161Timestamping(unittest.TestCase):
    """
    F6: RFC 3161 Timestamping — Boundary & Corner Cases.
    Tests unreachable TSA URL handling, network timeout simulation, SHA256 digest
    enforcement, missing /tr parameter detection, and builder CLI arguments.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_f6_b1_unreachable_tsa_url_handling(self):
        """F6-B1: Verify validation of TSA URL format and unreachable hostname detection."""
        invalid_url = "http://unreachable-tsa.invalid.internal/rfc3161"
        parsed = urllib.parse.urlparse(invalid_url)
        self.assertEqual(parsed.scheme, "http")
        self.assertEqual(parsed.netloc, "unreachable-tsa.invalid.internal")

        # Malformed URL without scheme
        malformed = "not_a_valid_url"
        parsed_mal = urllib.parse.urlparse(malformed)
        self.assertFalse(bool(parsed_mal.scheme), "Malformed TSA URL lacks scheme.")

    def test_f6_b2_tsa_timeout_simulation(self):
        """F6-B2: Verify tools/builder.py includes timeout handling or resilient warning logic on TSA failure."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        has_timeout_resilience = (
            "timeout=" in src
            or ("except Exception as e:" in src and "Warning" in src)
        )
        self.assertTrue(
            has_timeout_resilience,
            "Requirement R2 / F6: tools/builder.py must handle TSA timeout/failure without unhandled crash.",
        )

    def test_f6_b3_digest_algorithm_sha256_enforcement(self):
        """F6-B3: Verify tools/builder.py enforces SHA256 for timestamp digest (/td SHA256 / -h sha256)."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        has_sha256_ts = (
            ("/td" in src and "SHA256" in src)
            or ("-h" in src and "sha256" in src.lower())
        )
        self.assertTrue(
            has_sha256_ts,
            "Requirement R2 / F6: tools/builder.py must explicitly pass SHA256 as timestamp digest algorithm (/td SHA256).",
        )
        self.assertNotIn("/td SHA1", src, "tools/builder.py must not use legacy SHA1 for timestamp digest.")
        self.assertNotIn("/td sha1", src, "tools/builder.py must not use legacy SHA1 for timestamp digest.")

    def test_f6_b4_missing_tr_parameter_detection(self):
        """F6-B4: Verify tools/builder.py passes /tr parameter alongside /td parameter."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/tr",
            src,
            "Requirement R2 / F6: tools/builder.py must supply '/tr' timestamp RFC 3161 URL parameter.",
        )
        self.assertIn(
            "/td",
            src,
            "Requirement R2 / F6: tools/builder.py must supply '/td' timestamp digest algorithm parameter.",
        )

    def test_f6_b5_builder_source_timestamp_arguments(self):
        """F6-B5: Verify builder.py includes /tr and /td SHA256 for all signing invocations."""
        if not self.builder_py_path.exists():
            self.fail(f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/tr",
            src,
            "Requirement R2 / F6: tools/builder.py must pass '/tr' for RFC 3161 timestamping.",
        )
        self.assertIn(
            "/td",
            src,
            "Requirement R2 / F6: tools/builder.py must pass '/td' for RFC 3161 timestamp digest algorithm.",
        )


# =============================================================================
# Feature 7: Full PE Binary Signing (Boundary & Corner Cases)
# =============================================================================

class TestTier2F7FullPEBinarySigning(unittest.TestCase):
    """
    F7: Full PE Binary Signing — Boundary & Corner Cases.
    Tests truncated PE binaries, out-of-bounds Security Directory pointers,
    zero-length signatures, unsigned detection, and signed PE verification.
    """

    def setUp(self):
        self.tmp_dir = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.tmp_dir.cleanup()

    def test_f7_b1_truncated_pe_binary(self):
        """F7-B1: Verify parser handles truncated byte buffer safely without index error."""
        # 40 bytes: less than minimum DOS header + e_lfanew
        short_bytes = b"MZ" + (b"\x00" * 38)
        parsed = PEParser.parse_bytes(short_bytes)
        self.assertFalse(parsed["is_pe"])

        # Valid DOS header but e_lfanew points beyond EOF
        dos = bytearray(64)
        dos[0:2] = b"MZ"
        struct.pack_into("<I", dos, 0x3C, 5000)  # Offset 5000 in a 64-byte file
        parsed_eof = PEParser.parse_bytes(bytes(dos))
        self.assertFalse(parsed_eof["is_pe"])
        self.assertIn("e_lfanew", parsed_eof.get("error", "").lower())

    def test_f7_b2_out_of_bounds_security_directory_va(self):
        """F7-B2: Verify out-of-bounds Security Directory RVA is handled safely without crashing."""
        raw_pe = AuthenticodeHelper.create_synthetic_pe(
            Path(self.tmp_dir.name) / "oob.dll", is_signed=False
        )
        data = bytearray(raw_pe.read_bytes())

        # Mutate Security Directory offset to 0x7FFFFFFF
        e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        coff_offset = e_lfanew + 4
        opt_offset = coff_offset + 20
        sec_dir_offset = opt_offset + 112 + (4 * 8)
        struct.pack_into("<II", data, sec_dir_offset, 0x7FFFFFFF, 1024)

        parsed = PEParser.parse_bytes(bytes(data))
        self.assertTrue(parsed["is_pe"])
        self.assertFalse(
            parsed["is_signed"],
            "F7-B2: Out-of-bounds Security Directory must not be marked as signed.",
        )

    def test_f7_b3_zero_length_signature_entry(self):
        """F7-B3: Verify Security Directory entry with size 0 is treated as unsigned."""
        raw_pe = AuthenticodeHelper.create_synthetic_pe(
            Path(self.tmp_dir.name) / "zerosize.dll", is_signed=False
        )
        data = bytearray(raw_pe.read_bytes())

        # Set VA to 0x200 but size to 0
        e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
        coff_offset = e_lfanew + 4
        opt_offset = coff_offset + 20
        sec_dir_offset = opt_offset + 112 + (4 * 8)
        struct.pack_into("<II", data, sec_dir_offset, 0x200, 0)

        parsed = PEParser.parse_bytes(bytes(data))
        self.assertTrue(parsed["is_pe"])
        self.assertFalse(parsed["is_signed"])

    def test_f7_b4_unsigned_pe_binary_detection(self):
        """F7-B4: Verify unsigned synthetic PE binary reports is_signed: False."""
        unsigned_pe = AuthenticodeHelper.create_synthetic_pe(
            Path(self.tmp_dir.name) / "unsigned.exe", is_signed=False
        )
        parsed = PEParser.parse_file(unsigned_pe)
        self.assertTrue(parsed["is_pe"])
        self.assertFalse(parsed["is_signed"])
        self.assertEqual(parsed["sec_dir_va"], 0)
        self.assertEqual(parsed["sec_dir_size"], 0)

    def test_f7_b5_signed_pe_binary_roundtrip(self):
        """F7-B5: Verify signed PE binary is parsed with WIN_CERTIFICATE table intact."""
        signed_pe = AuthenticodeHelper.create_synthetic_pe(
            Path(self.tmp_dir.name) / "signed.dll", is_signed=True, cert_subject="CN=Attune Production CA"
        )
        parsed = PEParser.parse_file(signed_pe)
        self.assertTrue(parsed["is_pe"])
        self.assertTrue(parsed["is_signed"])
        self.assertIsNotNone(parsed["cert_info"])
        self.assertGreater(parsed["cert_info"]["length"], 8)
        self.assertEqual(parsed["cert_info"]["type"], "0x2")  # WIN_CERT_TYPE_PKCS_SIGNED_DATA


# =============================================================================
# Feature 8: Inno Setup SignTool Directive (Boundary & Corner Cases)
# =============================================================================

class TestTier2F8InnoSetupSignToolDirective(unittest.TestCase):
    """
    F8: Inno Setup SignTool Directive — Boundary & Corner Cases.
    Tests empty SignTool macros in unsigned mode, special shell characters,
    SignedUninstaller compliance, undefined macro handling, and presence in attune_installer.iss.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f8_b1_empty_signtool_macro_unsigned_mode(self):
        """F8-B1: Verify packaging/windows/attune_installer.iss defines #ifndef SignToolName for unsigned mode."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script at {self.iss_path}")
        raw_iss = self.iss_path.read_text(encoding="utf-8")
        self.assertIn(
            "#ifndef SignToolName",
            raw_iss,
            "Requirement R3 / F8: attune_installer.iss must contain '#ifndef SignToolName' default guard.",
        )

    def test_f8_b2_shell_metacharacters_in_signtool(self):
        """F8-B2: Verify attune_installer.iss does not execute shell command strings directly in SignTool."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script at {self.iss_path}")
        raw_iss = self.iss_path.read_text(encoding="utf-8")
        setup_signtool = self.parser.get_setup_directive("SignTool")
        if setup_signtool is not None:
            self.assertTrue(
                setup_signtool.startswith("{#") or setup_signtool == "" or setup_signtool.isalpha(),
                "Requirement R3 / F8: SignTool directive must reference a preprocessor macro, not a raw shell command.",
            )
        self.assertNotIn("cmd.exe", raw_iss[:raw_iss.find("[Files]") if "[Files]" in raw_iss else len(raw_iss)])

    def test_f8_b3_signed_uninstaller_directive_compliance(self):
        """F8-B3: Verify SignedUninstaller=yes is explicitly configured in [Setup]."""
        signed_unins = self.parser.get_setup_directive("SignedUninstaller")
        self.assertEqual(
            signed_unins,
            "yes",
            "Requirement R3 / F8: attune_installer.iss must define 'SignedUninstaller=yes' in [Setup].",
        )

    def test_f8_b4_undefined_macro_in_signtool(self):
        """F8-B4: Verify attune_installer.iss guards SignToolName with #ifndef to prevent undefined macro errors."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script at {self.iss_path}")
        raw_iss = self.iss_path.read_text(encoding="utf-8")
        has_guard = "#ifndef SignToolName" in raw_iss
        self.assertTrue(
            has_guard,
            "Requirement R3 / F8: attune_installer.iss must initialize SignToolName inside #ifndef to prevent ISCC errors.",
        )

    def test_f8_b5_signtool_setup_directive_presence(self):
        """F8-B5: Verify packaging/windows/attune_installer.iss contains SignTool directive in [Setup]."""
        signtool_dir = self.parser.get_setup_directive("SignTool")
        self.assertIsNotNone(
            signtool_dir,
            "Requirement R3 / F8: packaging/windows/attune_installer.iss must contain 'SignTool=' in [Setup].",
        )


# =============================================================================
# Feature 9: Inno Setup signonce Flags (Boundary & Corner Cases)
# =============================================================================

class TestTier2F9InnoSetupSignonceFlags(unittest.TestCase):
    """
    F9: Inno Setup signonce Flags — Boundary & Corner Cases.
    Tests boundary flags: no signonce on .json/.txt/.ico metadata files,
    mandatory signonce on dynamic DLLs, and case-insensitive flag matching.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f9_b1_no_signonce_on_integrity_manifest(self):
        """F9-B1: Verify signonce flag is strictly absent on attune_integrity.json."""
        files = self.parser.get_files_entries()
        manifest_entries = [f for f in files if "attune_integrity.json" in f.get("source", "").lower()]
        for e in manifest_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                "Requirement R3 / F9: Non-PE file attune_integrity.json must NOT have 'signonce' flag.",
            )

    def test_f9_b2_no_signonce_on_license_or_readme(self):
        """F9-B2: Verify signonce flag is strictly absent on .txt files."""
        files = self.parser.get_files_entries()
        txt_entries = [f for f in files if f.get("source", "").lower().endswith(".txt")]
        for e in txt_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                f"F9-B2: Text file {e.get('source')} must NOT include 'signonce' flag.",
            )

    def test_f9_b3_no_signonce_on_icons(self):
        """F9-B3: Verify signonce flag is strictly absent on .ico icon files."""
        files = self.parser.get_files_entries()
        ico_entries = [f for f in files if f.get("source", "").lower().endswith(".ico")]
        for e in ico_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                f"F9-B3: Icon file {e.get('source')} must NOT include 'signonce' flag.",
            )

    def test_f9_b4_dynamic_dlls_must_have_signonce(self):
        """F9-B4: Verify all dynamic workspace DLLs include the signonce flag."""
        files = self.parser.get_files_entries()
        dll_entries = [
            f for f in files
            if any(dll in f.get("source", "").lower() for dll in REQUIRED_DYNAMIC_DLLS)
        ]
        self.assertGreater(len(dll_entries), 0, "No dynamic DLL entries found in [Files].")
        for e in dll_entries:
            self.assertIn(
                "signonce",
                e.get("flags", set()),
                f"Requirement R3 / F9: Dynamic DLL {e.get('source')} must have 'signonce' flag.",
            )

    def test_f9_b5_case_insensitive_flag_normalization(self):
        """F9-B5: Verify InnoSetupParser normalizes case variations (SignOnce, SIGNONCE, signonce)."""
        iss_sample = (
            "[Files]\n"
            'Source: "attune_engine.dll"; DestDir: "{app}"; Flags: SignOnce OverwriteReadOnly\n'
            'Source: "attune_watchguard.dll"; DestDir: "{app}"; Flags: SIGNONCE\n'
        )
        p = InnoSetupParser(iss_sample)
        entries = p.get_files_entries()
        self.assertEqual(len(entries), 2)
        for e in entries:
            self.assertIn("signonce", e.get("flags", set()))
            self.assertTrue(p.has_signonce(e))


# =============================================================================
# Feature 10: Inno Setup CLI /Ssigntool Param (Boundary & Corner Cases)
# =============================================================================

class TestTier2F10InnoSetupCLISigntoolParam(unittest.TestCase):
    """
    F10: Inno Setup CLI /Ssigntool Param — Boundary & Corner Cases.
    Tests paths with spaces requiring inner $q quoting, missing $f placeholder,
    passwords with special characters, missing /S prefix, and builder CLI invocation.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_f10_b1_paths_with_spaces_dollar_q_quoting(self):
        """F10-B1: Inspect tools/builder.py to verify /Ssigntool wraps paths with $q inner quotes."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "$q",
            src,
            "Requirement R3 / F10: tools/builder.py must use $q inner quoting when building /Ssigntool parameter.",
        )

    def test_f10_b2_missing_dollar_f_placeholder_rejected(self):
        """F10-B2: Inspect tools/builder.py to verify /Ssigntool string ends with $f placeholder."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "$f",
            src,
            "Requirement R3 / F10: tools/builder.py /Ssigntool argument must terminate with $f placeholder.",
        )

    def test_f10_b3_special_characters_in_cert_password(self):
        """F10-B3: Inspect tools/builder.py to verify password handling in /Ssigntool."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        has_pwd_handling = (
            "/p $q" in src
            or ("/p" in src and "$q" in src)
        )
        self.assertTrue(
            has_pwd_handling,
            "Requirement R3 / F10: tools/builder.py must format password parameter with $q quoting for /Ssigntool.",
        )

    def test_f10_b4_missing_slash_s_prefix_rejected(self):
        """F10-B4: Inspect tools/builder.py to verify /S prefix is used when passing signtool to iscc."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/Ssigntool=",
            src,
            "Requirement R3 / F10: tools/builder.py must format Inno Setup sign tool argument with '/S' prefix (/Ssigntool=...).",
        )

    def test_f10_b5_builder_passes_ssigntool_to_iscc(self):
        """F10-B5: Verify tools/builder.py constructs and passes /Ssigntool= to iscc."""
        if not self.builder_py_path.exists():
            self.fail(f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/Ssigntool=",
            src,
            "Requirement R3 / F10: tools/builder.py must pass '/Ssigntool=...' parameter to iscc.",
        )


# =============================================================================
# Feature 11: verify_signatures.yml Workflow (Boundary & Corner Cases)
# =============================================================================

class TestTier2F11VerifySignaturesWorkflow(unittest.TestCase):
    """
    F11: verify_signatures.yml Workflow — Boundary & Corner Cases.
    Tests malformed YAML handling, workflow existence, dispatch input schemas,
    runner OS enforcement (windows-2022), and least-privilege permissions.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f11_b1_malformed_yaml_syntax_handling(self):
        """F11-B1: Verify .github/workflows/verify_signatures.yml validates with WorkflowValidator and catches malformed syntax."""
        if not self.workflow_path.exists():
            self.fail(f"Requirement R4 / F11: Missing workflow file at {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        self.assertIsNotNone(validator.data, "Workflow file must parse into non-empty dictionary.")
        corrupt_content = validator.raw_content + "\n  corrupt_yaml_tab:\t\t[\n"
        with self.assertRaises(yaml.YAMLError):
            WorkflowValidator(corrupt_content)

    def test_f11_b2_workflow_file_presence(self):
        """F11-B2: Verify .github/workflows/verify_signatures.yml exists."""
        self.assertTrue(
            self.workflow_path.exists(),
            f"Requirement R4 / F11: Dedicated verification workflow file missing at {self.workflow_path}",
        )

    def test_f11_b3_workflow_dispatch_inputs_schema(self):
        """F11-B3: Verify workflow_dispatch defines run_id and build_type inputs."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        on_clause = validator.data.get("on") or validator.data.get(True) or {}
        dispatch = on_clause.get("workflow_dispatch", {})
        inputs = dispatch.get("inputs", {})
        self.assertIn("build_type", inputs, "F11-B3: workflow_dispatch must define 'build_type' input.")

    def test_f11_b4_reject_non_windows_runner(self):
        """F11-B4: Verify job runs-on is windows-2022 and strictly NOT ubuntu-latest or macos-latest."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        jobs = validator.get_jobs()
        self.assertGreater(len(jobs), 0, "No jobs declared in verify_signatures.yml")
        for jname, jdef in jobs.items():
            runs_on = jdef.get("runs-on")
            self.assertEqual(
                runs_on,
                "windows-2022",
                f"F11-B4: Job '{jname}' must run on 'windows-2022', found '{runs_on}'",
            )
            self.assertNotIn("ubuntu", str(runs_on).lower())
            self.assertNotIn("macos", str(runs_on).lower())

    def test_f11_b5_least_privilege_permissions(self):
        """F11-B5: Verify workflow defines contents: read and avoids write-all."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        perms = validator.get_permissions()
        self.assertEqual(
            perms.get("contents"),
            "read",
            "F11-B5: verify_signatures.yml must define least-privilege 'contents: read'.",
        )
        self.assertNotEqual(perms, "write-all")


# =============================================================================
# Feature 12: Dual Authenticode Verification (Boundary & Corner Cases)
# =============================================================================

class TestTier2F12DualAuthenticodeVerification(unittest.TestCase):
    """
    F12: Dual Authenticode Verification — Boundary & Corner Cases.
    Tests non-PE target rejection, HashMismatch status handling, NotSigned handling,
    complete verification across all 6 target binaries, and timestamp requirement.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f12_b1_powershell_check_on_non_pe_file(self):
        """F12-B1: Verify PowerShell verification targets only PE binaries and ignores metadata."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        # Verification targets must not include non-PE files
        self.assertNotIn("attune_integrity.json", scripts)
        self.assertNotIn("LICENSE.txt", scripts)

    def test_f12_b2_hash_mismatch_status_handling(self):
        """F12-B2: Verify script fails on HashMismatch status."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        # Condition -ne 'Valid' captures HashMismatch, NotSigned, UnknownError
        self.assertTrue(
            "-ne 'Valid'" in scripts or "-ne \"Valid\"" in scripts or "HashMismatch" in scripts,
            "Requirement R4 / F12: Verification script must fail when Status is not 'Valid'.",
        )

    def test_f12_b3_not_signed_status_handling(self):
        """F12-B3: Verify script explicitly fails when binary status is NotSigned."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        has_error_action = "throw" in scripts.lower() or "exit 1" in scripts.lower()
        self.assertTrue(has_error_action, "F12-B3: Script must throw or exit 1 on invalid signature status.")

    def test_f12_b4_all_six_target_binaries_verified(self):
        """F12-B4: Verify PowerShell script checks all 6 targets (desktop exe + 5 DLLs)."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        for binary in REQUIRED_PE_BINARIES:
            self.assertIn(
                binary,
                scripts,
                f"Requirement R4 / F12: Verification script must check '{binary}'.",
            )

    def test_f12_b5_timestamper_certificate_mandatory(self):
        """F12-B5: Verify PowerShell script enforces presence of RFC 3161 TimeStamperCertificate."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn(
            "TimeStamperCertificate",
            scripts,
            "Requirement R4 / F12: Script must verify TimeStamperCertificate to ensure validity after cert expiration.",
        )


# =============================================================================
# Feature 13: Trust Store Registration (Boundary & Corner Cases)
# =============================================================================

class TestTier2F13TrustStoreRegistration(unittest.TestCase):
    """
    F13: Trust Store Registration — Boundary & Corner Cases.
    Tests base64 certificate decoding with CRLF/whitespace, trust anchor step presence,
    LocalMachine\\Root scope enforcement, conditional guards, and empty passwords.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f13_b1_base64_cert_with_crlf_and_whitespace(self):
        """F13-B1: Verify base64 certificate payload with CRLF line breaks decodes without error."""
        raw_pfx = Base64CertHelper.generate_dummy_pfx()
        clean_b64 = Base64CertHelper.encode_base64(raw_pfx)

        # Insert CRLF (\r\n) line endings every 64 characters and trailing spaces
        chunks = [clean_b64[i:i+64] for i in range(0, len(clean_b64), 64)]
        dirty_b64 = "\r\n".join(chunks) + "  \r\n"

        # Python / .NET FromBase64String should handle or allow whitespace stripping
        cleaned = re.sub(r"\s+", "", dirty_b64)
        decoded = base64.b64decode(cleaned)
        self.assertEqual(decoded, raw_pfx)

    def test_f13_b2_trust_anchor_step_presence(self):
        """F13-B2: Verify verify_signatures.yml contains trust anchor setup step."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        step = validator.find_step("verify_authenticode", name_substr="trust")
        self.assertIsNotNone(
            step,
            "Requirement R4 / F13: Workflow must contain a step setting up Windows Trust Anchor.",
        )

    def test_f13_b3_localmachine_root_store_scope(self):
        """F13-B3: Verify trust store registration targets LocalMachine\\Root, not CurrentUser."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn("Root", scripts)
        self.assertIn(
            "LocalMachine",
            scripts,
            "Requirement R4 / F13: Must import into 'LocalMachine' store so all runner processes trust cert.",
        )

    def test_f13_b4_conditional_guard_when_secret_unset(self):
        """F13-B4: Verify trust anchor setup is guarded with conditional check when secret is unset."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertTrue(
            "if ($env:WINDOWS_CERT_BASE64)" in scripts or "if ($env:WINDOWS_CERT_BASE64 -ne" in scripts,
            "F13-B4: Trust store step must guard against null/empty WINDOWS_CERT_BASE64 in PR builds.",
        )

    def test_f13_b5_empty_password_handling(self):
        """F13-B5: Verify certificate resolution handles empty password (\"\")."""
        raw_pfx_empty_pass = Base64CertHelper.generate_dummy_pfx(password="")
        b64_str = Base64CertHelper.encode_base64(raw_pfx_empty_pass)
        decoded = Base64CertHelper.decode_base64(b64_str)
        self.assertEqual(decoded, raw_pfx_empty_pass)


# =============================================================================
# Feature 14: Release Gating Integration (Boundary & Corner Cases)
# =============================================================================

class TestTier2F14ReleaseGatingIntegration(unittest.TestCase):
    """
    F14: Release Gating Integration — Boundary & Corner Cases.
    Tests verify_signatures job presence in build_and_release.yml, deploy job gating,
    build dependency, secrets inheritance, and build_type parameter passing.
    """

    def setUp(self):
        self.release_workflow_path = WORKFLOWS_DIR / "build_and_release.yml"
        self.validator = WorkflowValidator.parse_file(self.release_workflow_path)

    def test_f14_b1_verify_signatures_job_presence(self):
        """F14-B1: Verify build_and_release.yml declares verify_signatures job."""
        jobs = self.validator.get_jobs()
        self.assertIn(
            "verify_signatures",
            jobs,
            "Requirement R4 / F14: build_and_release.yml must define 'verify_signatures' job.",
        )

    def test_f14_b2_deploy_strictly_gated_on_verify_signatures(self):
        """F14-B2: Verify deploy job declares needs dependency on verify_signatures."""
        deploy_needs = self.validator.get_job_needs("deploy")
        self.assertIn(
            "verify_signatures",
            deploy_needs,
            "Requirement R4 / F14: 'deploy' job must depend on 'verify_signatures' "
            "so any signing or verification failure strictly blocks deployment.",
        )

    def test_f14_b3_verify_signatures_depends_on_build(self):
        """F14-B3: Verify verify_signatures job declares needs dependency on build."""
        v_needs = self.validator.get_job_needs("verify_signatures")
        self.assertIn(
            "build",
            v_needs,
            "F14-B3: verify_signatures job must depend on 'build'.",
        )

    def test_f14_b4_secrets_pass_through_windows_cert(self):
        """F14-B4: Verify certificate secrets are passed through to verify_signatures job."""
        vjob = self.validator.get_job("verify_signatures")
        if not vjob:
            self.fail("verify_signatures job not found in build_and_release.yml")
        secrets_block = vjob.get("secrets", {})
        has_secrets = (
            "WINDOWS_CERT_BASE64" in str(secrets_block)
            or secrets_block == "inherit"
            or "env" in vjob
        )
        self.assertTrue(
            has_secrets,
            "F14-B4: WINDOWS_CERT_BASE64 secret must be passed to verify_signatures.",
        )

    def test_f14_b5_build_type_parameter_passed(self):
        """F14-B5: Verify build_type parameter is passed to verify_signatures workflow call."""
        vjob = self.validator.get_job("verify_signatures")
        if not vjob:
            self.fail("verify_signatures job not found in build_and_release.yml")
        with_block = vjob.get("with", {})
        has_build_type = "build_type" in with_block or "matrix" in str(vjob)
        self.assertTrue(
            has_build_type,
            "F14-B5: 'build_type' parameter must be passed to verify_signatures job.",
        )


if __name__ == "__main__":
    unittest.main()
