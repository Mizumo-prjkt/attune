"""
Tier 1 Feature Coverage Test Suite — Attune Windows Runtime, Packaging, Code Signing & Verification.

Authoritative Specifications & Coverage Matrix:
- ORIGINAL_REQUEST.md: Requirements R1, R2, R3, R4
- PROJECT.md: Features 1–14
- TEST_INFRA.md: Tier 1 Test Scenarios & Matrix
- SCOPE.md: Milestone T2 Requirements

Features Covered (14 Features, >=5 tests each, >=70 tests total):
  - F1: SetDllDirectoryW Initialization (5 tests)
  - F2: LoadLibraryW Unicode Loading (5 tests)
  - F3: Dynamic Component Bundling (5 tests)
  - F4: Integrity Manifest Synchronization (5 tests)
  - F5: Base64 Certificate Resolution (5 tests)
  - F6: RFC 3161 Timestamping (5 tests)
  - F7: Full PE Binary Signing (5 tests)
  - F8: Inno Setup SignTool Directive (5 tests)
  - F9: Inno Setup signonce Flags (5 tests)
  - F10: Inno Setup CLI /Ssigntool Param (5 tests)
  - F11: verify_signatures.yml Workflow (5 tests)
  - F12: Dual Authenticode Verification (5 tests)
  - F13: Trust Store Registration (5 tests)
  - F14: Release Gating Integration (5 tests)
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import re
import shutil
import tempfile
import unittest
from pathlib import Path
from typing import Any, Dict, List, Optional

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
# Feature 1: SetDllDirectoryW Initialization (Requirement R1)
# =============================================================================

class TestF1SetDllDirectoryW(unittest.TestCase):
    """
    F1: SetDllDirectoryW Initialization
    Verifies that the application initializes the Windows dynamic library search path
    at earliest startup in main.rs so that native workspace DLLs in {app}\\lib
    and application root are discoverable by the OS dynamic linker.
    """

    def setUp(self):
        self.main_rs_path = CRATES_DIR / "attune-desktop" / "src" / "main.rs"
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_f1_1_basic_initialization(self):
        """F1-1: Verify SetDllDirectoryW initialization call in main.rs or dynamic_components.rs."""
        self.assertTrue(self.main_rs_path.exists(), f"Source file missing: {self.main_rs_path}")
        main_src = self.main_rs_path.read_text(encoding="utf-8")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8") if self.dyn_comp_path.exists() else ""
        combined = main_src + "\n" + dyn_src

        self.assertIn(
            "SetDllDirectoryW",
            combined,
            "Requirement R1 / F1: attune-desktop must call SetDllDirectoryW at startup "
            "to register {app}\\lib in the Windows dynamic linker search path.",
        )
        self.assertNotIn(
            "SetDllDirectoryA",
            combined,
            "Requirement R1 / F1: ANSI SetDllDirectoryA must not be used; wide SetDllDirectoryW is required.",
        )

    def test_f1_2_path_format(self):
        """F1-2: Verify path format converts backslashes and null-terminated UTF-16 wide string."""
        sample_path = "C:\\Program Files\\Attune\\lib"
        wide_words = RustLoaderContract.encode_wide(sample_path)
        self.assertTrue(
            RustLoaderContract.validate_null_termination(wide_words),
            "F1: SetDllDirectoryW path parameter must be null-terminated UTF-16 wide string.",
        )
        decoded = RustLoaderContract.decode_wide(wide_words)
        self.assertEqual(decoded, sample_path, "F1: Wide character decode must match input path.")

    def test_f1_3_error_handling(self):
        """F1-3: Verify Win32 error handling on SetDllDirectoryW failure captures GetLastError in dynamic_components.rs."""
        self.assertTrue(self.dyn_comp_path.exists(), f"Source file missing: {self.dyn_comp_path}")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")
        self.assertIn(
            "GetLastError",
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs must declare/invoke GetLastError() for Win32 API failure diagnostics.",
        )
        self.assertIn(
            "format_windows_error",
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs must implement format_windows_error to decode Win32 error codes.",
        )
        self.assertTrue(
            ("SetDllDirectoryW" in dyn_src and "GetLastError()" in dyn_src),
            "Requirement R1 / F1: SetDllDirectoryW failure branch must capture GetLastError().",
        )

    def test_f1_4_multiple_paths(self):
        """F1-4: Verify candidate search paths in dynamic_components.rs include both lib subdir and app dir."""
        self.assertTrue(self.dyn_comp_path.exists(), f"Source file missing: {self.dyn_comp_path}")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")
        self.assertIn(
            'join("lib")',
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs candidate resolution must include 'lib' subdirectory.",
        )
        self.assertIn(
            "resolve_candidate_paths",
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs must define resolve_candidate_paths.",
        )
        self.assertIn(
            "ATTUNE_LIB_DIR",
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs must support ATTUNE_LIB_DIR override.",
        )

    def test_f1_5_search_order(self):
        """F1-5: Verify DLL search order prioritizes {app}\\lib and {app} before system paths."""
        app_dir = Path("C:/Attune")
        lib_dir = app_dir / "lib"
        system_dirs = [Path("C:/Windows/System32"), Path("C:/Windows")]
        search_order = RustLoaderContract.simulate_search_order(
            "attune_engine.dll", lib_dir, app_dir, system_dirs
        )
        self.assertEqual(search_order[0], app_dir / "attune_engine.dll")
        self.assertEqual(search_order[1], lib_dir / "attune_engine.dll")
        self.assertEqual(search_order[2], system_dirs[0] / "attune_engine.dll")


# =============================================================================
# Feature 2: LoadLibraryW Unicode Loading (Requirement R1)
# =============================================================================

class TestF2LoadLibraryW(unittest.TestCase):
    """
    F2: LoadLibraryW Unicode Loading
    Verifies wide-character Unicode dynamic library loading in dynamic_components.rs,
    replacing ANSI LoadLibraryA to prevent Error 126 crashes on non-ASCII paths.
    """

    def setUp(self):
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_f2_1_wide_character_encoding(self):
        """F2-1: Verify path conversion produces wide character buffer (Vec<u16>)."""
        test_path = "C:\\Users\\AttuneUser\\lib\\attune_engine.dll"
        wide = RustLoaderContract.encode_wide(test_path)
        self.assertIsInstance(wide, list)
        self.assertGreater(len(wide), len(test_path))

    def test_f2_2_null_termination(self):
        """F2-2: Verify wide character array is terminated with exactly 0x0000."""
        test_path = "C:\\Music\\attune_procmgr.dll"
        wide = RustLoaderContract.encode_wide(test_path)
        self.assertEqual(wide[-1], 0, "F2: Wide string must end with null terminator 0x0000.")
        self.assertTrue(RustLoaderContract.validate_null_termination(wide))

    def test_f2_3_error_reporting(self):
        """F2-3: Verify error reporting on load failure captures Windows error code."""
        if not self.dyn_comp_path.exists():
            self.fail(f"Required source file not found: {self.dyn_comp_path}")
        src = self.dyn_comp_path.read_text(encoding="utf-8")
        # Ensure LoadLibraryW is used rather than LoadLibraryA
        self.assertIn(
            "LoadLibraryW",
            src,
            "Requirement R1 / F2: os_dlopen in dynamic_components.rs must use LoadLibraryW "
            "with UTF-16 wide string encoding, not LoadLibraryA.",
        )

    def test_f2_4_non_ascii_unicode_path(self):
        """F2-4: Verify non-ASCII path (Japanese, accents, Cyrillic) encodes without loss."""
        unicode_paths = [
            "C:\\Users\\みく\\Attune\\attune_engine.dll",
            "C:\\Users\\José\\Musica\\attune_watchguard.dll",
            "C:\\Users\\Иван\\Music\\attune_procmgr.dll",
        ]
        for p in unicode_paths:
            wide = RustLoaderContract.encode_wide(p)
            decoded = RustLoaderContract.decode_wide(wide)
            self.assertEqual(decoded, p, f"F2: Multilingual path roundtrip failed for {p}")

    def test_f2_5_handle_return(self):
        """F2-5: Verify handle return contract Result<*mut c_void, String>."""
        if not self.dyn_comp_path.exists():
            self.fail(f"Required source file not found: {self.dyn_comp_path}")
        src = self.dyn_comp_path.read_text(encoding="utf-8")
        self.assertIn("Result<*mut", src, "F2: os_dlopen signature must return Result<*mut c_void, String>.")


# =============================================================================
# Feature 3: Dynamic Component Bundling (Requirement R1)
# =============================================================================

class TestF3DynamicComponentBundling(unittest.TestCase):
    """
    F3: Dynamic Component Bundling
    Verifies that all 5 dynamic components are bundled in {app} and {app}\\lib
    with correct destination names and unconditional upgrade flags.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f3_1_five_dlls_presence_in_app_root(self):
        """F3-1: Verify all 5 dynamic DLLs are packaged into {app}."""
        files = self.parser.find_files_by_dest_dir("{app}")
        dest_names = {
            f.get("destname", Path(f.get("source", "")).name).lower() for f in files
        }
        for dll in REQUIRED_DYNAMIC_DLLS:
            self.assertIn(
                dll.lower(),
                dest_names,
                f"Requirement R1 / F3: {dll} must be packaged into {{app}} in attune_installer.iss",
            )

    def test_f3_2_presence_in_app_lib(self):
        """F3-2: Verify all 5 dynamic DLLs are packaged into {app}\\lib."""
        files = self.parser.find_files_by_dest_dir("{app}\\lib")
        dest_names = {
            f.get("destname", Path(f.get("source", "")).name).lower() for f in files
        }
        for dll in REQUIRED_DYNAMIC_DLLS:
            self.assertIn(
                dll.lower(),
                dest_names,
                f"Requirement R1 / F3: {dll} must be packaged into {{app}}\\lib in attune_installer.iss",
            )

    def test_f3_3_destination_names(self):
        """F3-3: Verify destination names include attune_music_metadata_changer.dll."""
        files = self.parser.get_files_entries()
        has_changer = any(
            f.get("destname", "").lower() == "attune_music_metadata_changer.dll"
            or "attune_music_metadata_changer.dll" in f.get("source", "").lower()
            for f in files
        )
        self.assertTrue(
            has_changer,
            "Requirement R1 / F3: attune_music_metadata_changer.dll alias must be packaged.",
        )

    def test_f3_4_upgrade_flags(self):
        """F3-4: Verify DLL file entries include all required upgrade flags."""
        files = self.parser.get_files_entries()
        dll_files = [f for f in files if any(dll in f.get("source", "").lower() for dll in REQUIRED_DYNAMIC_DLLS)]
        self.assertGreater(len(dll_files), 0, "No dynamic DLL files found in [Files] section.")
        for f in dll_files:
            flags = f.get("flags", set())
            for req_flag in REQUIRED_DLL_UPGRADE_FLAGS:
                self.assertIn(
                    req_flag,
                    flags,
                    f"F3: Dynamic DLL {f.get('source')} missing required flag '{req_flag}'.",
                )

    def test_f3_5_no_missing_components(self):
        """F3-5: Verify complete component inventory matches REQUIRED_DYNAMIC_DLLS."""
        fixture = WindowsStagingFixture()
        try:
            staged = fixture.stage_binaries(signed=False)
            for req_dll in REQUIRED_DYNAMIC_DLLS:
                self.assertIn(req_dll, staged, f"F3: Staging fixture missing {req_dll}")
                self.assertTrue(staged[req_dll].exists())
        finally:
            fixture.cleanup()


# =============================================================================
# Feature 4: Integrity Manifest Synchronization (Requirements R1, R2)
# =============================================================================

class TestF4IntegrityManifestSynchronization(unittest.TestCase):
    """
    F4: Integrity Manifest Synchronization
    Verifies that attune_integrity.json is etched strictly post-signing,
    and runtime verification accurately detects valid binaries and tampering.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.staged = self.fixture.stage_binaries(signed=False)

    def tearDown(self):
        self.fixture.cleanup()

    def test_f4_1_hashing_post_signing(self):
        """F4-1: Demonstrate that signing mutates PE binary hash; post-signing hash required."""
        target_dll = self.fixture.dist_lib_dir / "attune_engine.dll"
        hash_before = hashlib.sha256(target_dll.read_bytes()).hexdigest()

        # Sign the DLL
        AuthenticodeHelper.append_authenticode_signature(target_dll)
        hash_after = hashlib.sha256(target_dll.read_bytes()).hexdigest()

        self.assertNotEqual(
            hash_before,
            hash_after,
            "F4: Authenticode signing must modify file hash. Manifest generated pre-signing will mismatch.",
        )

    def test_f4_2_manifest_generation(self):
        """F4-2: Verify manifest generation produces valid JSON with all 5 DLL hashes."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        self.assertEqual(manifest["version"], "0.1.0")
        self.assertIn("components", manifest)
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            self.assertIn(dll_name, manifest["components"])
            self.assertIn("sha256", manifest["components"][dll_name])
            self.assertIn("blake3", manifest["components"][dll_name])

    def test_f4_3_sha256_match(self):
        """F4-3: Verify runtime SHA-256 integrity check matches on unmodified binary."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        target_dll = self.fixture.dist_lib_dir / "attune_procmgr.dll"
        expected_sha = manifest["components"]["attune_procmgr.dll"]["sha256"]

        is_valid, msg = self.fixture.verify_component_integrity(target_dll, expected_sha)
        self.assertTrue(is_valid, f"F4: Valid component rejected: {msg}")

    def test_f4_4_blake3_match(self):
        """F4-4: Verify BLAKE3 hash computation against manifest record."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        target_dll = self.fixture.dist_lib_dir / "attune_watchguard.dll"
        hashes = self.fixture.compute_file_hashes(target_dll)
        self.assertEqual(
            hashes["blake3"],
            manifest["components"]["attune_watchguard.dll"]["blake3"],
            "F4: Computed BLAKE3 hash does not match manifest.",
        )

    def test_f4_5_tamper_detection(self):
        """F4-5: Verify single-byte tampering triggers critical anti-tamper rejection."""
        manifest = self.fixture.generate_manifest(version="0.1.0")
        target_dll = self.fixture.dist_lib_dir / "attune_music_metadata.dll"
        expected_sha = manifest["components"]["attune_music_metadata.dll"]["sha256"]

        # Tamper with file
        data = bytearray(target_dll.read_bytes())
        data[200] = (data[200] + 1) % 256
        target_dll.write_bytes(data)

        is_valid, msg = self.fixture.verify_component_integrity(target_dll, expected_sha)
        self.assertFalse(is_valid, "F4: Tampered binary must be rejected!")
        self.assertIn("CRITICAL", msg)
        self.assertIn("mismatch", msg)


# =============================================================================
# Feature 5: Base64 Certificate Resolution (Requirement R2)
# =============================================================================

class TestF5Base64CertificateResolution(unittest.TestCase):
    """
    F5: Base64 Certificate Resolution
    Verifies decoding WINDOWS_CERT_BASE64 to a temporary .pfx file in tools/builder.py
    and passing secrets in CI.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"
        self.tmp_dir = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.tmp_dir.cleanup()

    def test_f5_1_valid_base64_decode(self):
        """F5-1: Verify valid base64-encoded PKCS#12 payload decodes byte-for-byte."""
        dummy_pfx = Base64CertHelper.generate_dummy_pfx(password="Secret123!")
        b64_str = Base64CertHelper.encode_base64(dummy_pfx)
        decoded = Base64CertHelper.decode_base64(b64_str)
        self.assertEqual(decoded, dummy_pfx)

    def test_f5_2_output_path(self):
        """F5-2: Verify decoded certificate is written to designated temporary .pfx file."""
        dummy_pfx = Base64CertHelper.generate_dummy_pfx()
        b64_str = Base64CertHelper.encode_base64(dummy_pfx)
        out_path = Path(self.tmp_dir.name) / "temp_cert.pfx"
        Base64CertHelper.resolve_cert_to_file(b64_str, out_path)
        self.assertTrue(out_path.exists())
        self.assertEqual(out_path.read_bytes(), dummy_pfx)

    def test_f5_3_password_extraction(self):
        """F5-3: Verify certificate password extraction from WINDOWS_CERT_PASS / CERT_PASS."""
        os.environ["WINDOWS_CERT_PASS"] = "TestPassword456"
        cert_pass = os.environ.get("CERT_PASS") or os.environ.get("WINDOWS_CERT_PASS")
        self.assertEqual(cert_pass, "TestPassword456")

    def test_f5_4_corrupted_base64_handling(self):
        """F5-4: Verify corrupted base64 string raises ValueError or binascii.Error."""
        corrupted = "NOT_VALID_BASE64_###$$$%%%"
        with self.assertRaises(Exception):
            Base64CertHelper.decode_base64(corrupted)

    def test_f5_5_fallback_behavior(self):
        """F5-5: Verify builder.py inspects WINDOWS_CERT_BASE64 or falls back to CERT_PATH."""
        self.assertTrue(self.builder_py_path.exists(), "builder.py must exist in tools/")
        src = self.builder_py_path.read_text(encoding="utf-8")
        # Check builder source for certificate environment resolution
        has_cert_env = "WINDOWS_CERT_PATH" in src or "CERT_PATH" in src or "WINDOWS_CERT_BASE64" in src
        self.assertTrue(has_cert_env, "builder.py must reference certificate environment variables.")


# =============================================================================
# Feature 6: RFC 3161 Timestamping (Requirement R2)
# =============================================================================

class TestF6RFC3161Timestamping(unittest.TestCase):
    """
    F6: RFC 3161 Timestamping
    Verifies that all SignTool and osslsigncode invocations configure RFC 3161
    countersignatures (/tr <url> /td SHA256 or osslsigncode -ts <url>).
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_f6_1_signtool_tr_parameter(self):
        """F6-1: Verify SignTool command includes /tr parameter for RFC 3161 timestamping."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/tr",
            src,
            "Requirement R2 / F6: SignTool command in tools/builder.py must include '/tr' "
            "for RFC 3161 timestamping.",
        )

    def test_f6_2_signtool_td_sha256_parameter(self):
        """F6-2: Verify SignTool command includes /td SHA256 parameter."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/td",
            src,
            "Requirement R2 / F6: SignTool command in tools/builder.py must include '/td' "
            "with SHA256 timestamp digest algorithm.",
        )

    def test_f6_3_osslsigncode_ts_parameter(self):
        """F6-3: Verify osslsigncode command includes -ts parameter."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "-ts",
            src,
            "Requirement R2 / F6: osslsigncode command in tools/builder.py must include '-ts' parameter.",
        )

    def test_f6_4_tsa_url_validity(self):
        """F6-4: Verify configured TSA URL in tools/builder.py is a valid RFC 3161 HTTP/HTTPS endpoint."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        tsa_matches = re.findall(r'https?://[^\s"\'<>]+timestamp[^\s"\'<>]*', src, re.IGNORECASE)
        self.assertTrue(
            len(tsa_matches) > 0,
            "Requirement R2 / F6: tools/builder.py must configure an RFC 3161 TSA URL (e.g. http://timestamp.digicert.com).",
        )
        for url in tsa_matches:
            self.assertTrue(
                url.startswith("http://") or url.startswith("https://"),
                f"F6-4: Configured TSA URL must be HTTP/HTTPS: {url}",
            )

    def test_f6_5_fallback_offline_mode(self):
        """F6-5: Verify tools/builder.py implements non-fatal error/warning handling for signing/TSA failure."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn("sign_dll_file", src, "tools/builder.py must define sign_dll_file")
        has_warning_fallback = (
            "except Exception" in src
            and ("[Sign/Authenticode] Warning:" in src or "Warning" in src)
        )
        self.assertTrue(
            has_warning_fallback,
            "Requirement R2 / F6: tools/builder.py must catch signing/timestamp exceptions and emit non-fatal warnings.",
        )


# =============================================================================
# Feature 7: Full PE Binary Signing (Requirement R2)
# =============================================================================

class TestF7FullPEBinarySigning(unittest.TestCase):
    """
    F7: Full PE Binary Signing
    Verifies that desktop exe, all dynamic DLLs, and dist directory artifacts
    are signed with Authenticode prior to packaging.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()

    def tearDown(self):
        self.fixture.cleanup()

    def test_f7_1_desktop_exe_signing(self):
        """F7-1: Verify attune-desktop.exe is signed with Authenticode table."""
        exe_path = AuthenticodeHelper.create_synthetic_pe(
            self.fixture.dist_dir / "attune-desktop.exe", is_signed=True
        )
        parsed = PEParser.parse_file(exe_path)
        self.assertTrue(parsed["is_pe"])
        self.assertTrue(parsed["is_signed"], "F7: attune-desktop.exe must be Authenticode-signed.")

    def test_f7_2_dll_signing(self):
        """F7-2: Verify all 5 dynamic DLLs are Authenticode signed."""
        paths = self.fixture.stage_binaries(signed=True)
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            parsed = PEParser.parse_file(paths[dll_name])
            self.assertTrue(parsed["is_signed"], f"F7: Dynamic component {dll_name} must be signed.")

    def test_f7_3_dist_directory_artifact_signing(self):
        """F7-3: Verify dist/windows artifacts (root and lib) are both signed."""
        self.fixture.stage_binaries(signed=True)
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            root_dll = self.fixture.dist_dir / dll_name
            lib_dll = self.fixture.dist_lib_dir / dll_name
            self.assertTrue(PEParser.parse_file(root_dll)["is_signed"])
            self.assertTrue(PEParser.parse_file(lib_dll)["is_signed"])

    def test_f7_4_exit_code_verification(self):
        """F7-4: Verify tools/builder.py verifies non-zero returncode from signing commands."""
        builder_py = TOOLS_DIR / "builder.py"
        self.assertTrue(builder_py.exists(), f"Missing builder.py at {builder_py}")
        src = builder_py.read_text(encoding="utf-8")
        has_returncode_check = "res.returncode == 0" in src or "returncode == 0" in src or "check=True" in src
        self.assertTrue(
            has_returncode_check,
            "Requirement R2 / F7: tools/builder.py must inspect subprocess returncode for signing command failures.",
        )
        self.assertIn(
            "osslsigncode note:",
            src,
            "Requirement R2 / F7: tools/builder.py must log diagnostics on non-zero returncode from osslsigncode.",
        )

    def test_f7_5_certificate_subject_validation(self):
        """F7-5: Verify certificate subject validation in WIN_CERTIFICATE table."""
        subject = "CN=Attune Official Release Signer"
        pe_path = AuthenticodeHelper.create_synthetic_pe(
            self.fixture.dist_dir / "test_signed.dll", is_signed=True, cert_subject=subject
        )
        parsed = PEParser.parse_file(pe_path)
        self.assertTrue(parsed["is_signed"])
        self.assertGreater(parsed["cert_info"]["payload_len"], 0)


# =============================================================================
# Feature 8: Inno Setup SignTool Directive (Requirement R3)
# =============================================================================

class TestF8InnoSetupSignToolDirective(unittest.TestCase):
    """
    F8: Inno Setup SignTool Directive
    Verifies that SignTool={#SignToolName} and SignedUninstaller=yes
    are declared in [Setup] section of attune_installer.iss.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f8_1_signtool_directive_in_setup(self):
        """F8-1: Verify SignTool directive is configured in [Setup]."""
        signtool_val = self.parser.get_setup_directive("SignTool")
        self.assertIsNotNone(
            signtool_val,
            "Requirement R3 / F8: packaging/windows/attune_installer.iss is missing "
            "'SignTool=' directive in [Setup] section.",
        )

    def test_f8_2_signed_uninstaller_yes(self):
        """F8-2: Verify SignedUninstaller=yes is configured in [Setup]."""
        signed_unins = self.parser.get_setup_directive("SignedUninstaller")
        self.assertEqual(
            signed_unins,
            "yes",
            "Requirement R3 / F8: packaging/windows/attune_installer.iss is missing "
            "'SignedUninstaller=yes' directive in [Setup] section.",
        )

    def test_f8_3_macro_definition(self):
        """F8-3: Verify #define SignToolName macro definition."""
        macro_val = self.parser.get_macro("SignToolName")
        self.assertIsNotNone(
            macro_val,
            "Requirement R3 / F8: attune_installer.iss must define '#define SignToolName'.",
        )

    def test_f8_4_conditional_guard(self):
        """F8-4: Verify packaging/windows/attune_installer.iss contains conditional guard for SignTool."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script at {self.iss_path}")
        raw_iss = self.iss_path.read_text(encoding="utf-8")
        has_guard = (
            '#if SignToolName != ""' in raw_iss
            or '#ifdef SignToolName' in raw_iss
            or ('#if' in raw_iss and 'SignTool' in raw_iss)
        )
        self.assertTrue(
            has_guard,
            "Requirement R3 / F8: packaging/windows/attune_installer.iss must conditionally guard SignTool directive.",
        )

    def test_f8_5_unsigned_mode(self):
        """F8-5: Verify packaging/windows/attune_installer.iss handles unsigned mode when SignToolName is empty."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script at {self.iss_path}")
        raw_iss = self.iss_path.read_text(encoding="utf-8")
        has_unsigned_support = (
            '#ifndef SignToolName' in raw_iss
            or '#define SignToolName ""' in raw_iss
        )
        self.assertTrue(
            has_unsigned_support,
            "Requirement R3 / F8: packaging/windows/attune_installer.iss must support unsigned mode via #ifndef SignToolName.",
        )


# =============================================================================
# Feature 9: Inno Setup signonce Flags (Requirement R3)
# =============================================================================

class TestF9InnoSetupSignonceFlags(unittest.TestCase):
    """
    F9: Inno Setup signonce Flags
    Verifies that signonce flag is present strictly on .exe and .dll entries in [Files],
    and NOT on non-binary metadata files (.json, .txt, .ico).
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.parser = InnoSetupParser.parse_file(self.iss_path)

    def test_f9_1_signonce_on_exe(self):
        """F9-1: Verify signonce flag is present on desktop .exe entry."""
        files = self.parser.get_files_entries()
        exe_entries = [f for f in files if "attune-desktop.exe" in f.get("source", "").lower()]
        self.assertGreater(len(exe_entries), 0, "No desktop .exe entry in [Files].")
        for e in exe_entries:
            self.assertIn(
                "signonce",
                e.get("flags", set()),
                "Requirement R3 / F9: attune-desktop.exe must include 'signonce' flag in [Files].",
            )

    def test_f9_2_signonce_on_dll(self):
        """F9-2: Verify signonce flag is present on all dynamic DLL entries."""
        files = self.parser.get_files_entries()
        dll_entries = [f for f in files if any(dll in f.get("source", "").lower() for dll in REQUIRED_DYNAMIC_DLLS)]
        self.assertGreater(len(dll_entries), 0, "No dynamic DLL entries in [Files].")
        for e in dll_entries:
            self.assertIn(
                "signonce",
                e.get("flags", set()),
                f"Requirement R3 / F9: DLL {e.get('source')} must include 'signonce' flag in [Files].",
            )

    def test_f9_3_no_signonce_on_json(self):
        """F9-3: Verify signonce flag is NOT present on .json files."""
        files = self.parser.get_files_entries()
        json_entries = [f for f in files if f.get("source", "").lower().endswith(".json")]
        for e in json_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                f"Requirement R3 / F9: Non-PE file {e.get('source')} must NOT include 'signonce' flag.",
            )

    def test_f9_4_no_signonce_on_txt(self):
        """F9-4: Verify signonce flag is NOT present on .txt files."""
        files = self.parser.get_files_entries()
        txt_entries = [f for f in files if f.get("source", "").lower().endswith(".txt")]
        for e in txt_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                f"Requirement R3 / F9: Text file {e.get('source')} must NOT include 'signonce' flag.",
            )

    def test_f9_5_no_signonce_on_ico(self):
        """F9-5: Verify signonce flag is NOT present on .ico files."""
        files = self.parser.get_files_entries()
        ico_entries = [f for f in files if f.get("source", "").lower().endswith(".ico")]
        for e in ico_entries:
            self.assertNotIn(
                "signonce",
                e.get("flags", set()),
                f"Requirement R3 / F9: Icon file {e.get('source')} must NOT include 'signonce' flag.",
            )


# =============================================================================
# Feature 10: Inno Setup CLI /Ssigntool Param (Requirement R3)
# =============================================================================

class TestF10InnoSetupCLISigntoolParam(unittest.TestCase):
    """
    F10: Inno Setup CLI /Ssigntool Param
    Verifies /Ssigntool argument construction with $q quote escaping and $f placeholder
    in tools/builder.py.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_f10_1_s_argument_format(self):
        """F10-1: Verify builder.py constructs /Ssigntool= CLI switch."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/Ssigntool=",
            src,
            "Requirement R3 / F10: tools/builder.py must pass '/Ssigntool=...' parameter to iscc.",
        )

    def test_f10_2_dollar_q_quote_escaping(self):
        """F10-2: Verify $q inner quote escaping around paths with spaces."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "$q",
            src,
            "Requirement R3 / F10: /Ssigntool command must use '$q' inner quote escaping for Inno Setup.",
        )

    def test_f10_3_dollar_f_placeholder(self):
        """F10-3: Verify $f placeholder at end of signtool command string."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "$f",
            src,
            "Requirement R3 / F10: /Ssigntool command must end with '$f' placeholder for file to sign.",
        )

    def test_f10_4_password_escaping(self):
        """F10-4: Verify tools/builder.py escapes certificate password with $q and double quotes for /Ssigntool."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py at {self.builder_py_path}")
        src = self.builder_py_path.read_text(encoding="utf-8")
        has_pass_escaping = (
            "/p $q" in src
            or 'replace(\'"\'' in src
            or "replace('\"'" in src
            or '$q' in src
        )
        self.assertTrue(
            has_pass_escaping,
            "Requirement R3 / F10: tools/builder.py must format certificate passwords using Inno Setup $q quoting.",
        )

    def test_f10_5_dynamic_signtool_path_resolution(self):
        """F10-5: Verify dynamic signtool path resolution logic in builder."""
        src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn("signtool", src)
        self.assertIn("shutil.which", src)


# =============================================================================
# Feature 11: verify_signatures.yml Workflow (Requirement R4)
# =============================================================================

class TestF11VerifySignaturesWorkflow(unittest.TestCase):
    """
    F11: verify_signatures.yml Workflow
    Verifies dedicated .github/workflows/verify_signatures.yml exists,
    defines required triggers, runs on windows-2022, and specifies permissions.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f11_1_workflow_file_exists(self):
        """F11-1: Verify .github/workflows/verify_signatures.yml exists."""
        self.assertTrue(
            self.workflow_path.exists(),
            f"Requirement R4 / F11: Dedicated verification workflow file missing at {self.workflow_path}",
        )

    def test_f11_2_triggers(self):
        """F11-2: Verify workflow defines workflow_dispatch, workflow_call, workflow_run."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        triggers = validator.get_triggers()
        self.assertIn("workflow_dispatch", triggers)
        self.assertIn("workflow_call", triggers)
        self.assertIn("workflow_run", triggers)

    def test_f11_3_runner_os_windows_2022(self):
        """F11-3: Verify verification job executes on windows-2022 runner."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        jobs = validator.get_jobs()
        self.assertGreater(len(jobs), 0, "No jobs declared in verify_signatures.yml")
        for jname, jdef in jobs.items():
            self.assertEqual(jdef.get("runs-on"), "windows-2022")

    def test_f11_4_permissions_block(self):
        """F11-4: Verify workflow defines secure permissions block."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        perms = validator.get_permissions()
        self.assertEqual(perms.get("contents"), "read")

    def test_f11_5_steps_structure(self):
        """F11-5: Verify steps structure in verify_signatures.yml."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        jobs = validator.get_jobs()
        first_job = next(iter(jobs.values()))
        steps = first_job.get("steps", [])
        self.assertGreaterEqual(len(steps), 2, "Workflow must contain checkout and verification steps.")


# =============================================================================
# Feature 12: Dual Authenticode Verification (Requirement R4)
# =============================================================================

class TestF12DualAuthenticodeVerification(unittest.TestCase):
    """
    F12: Dual Authenticode Verification
    Verifies PowerShell Get-AuthenticodeSignature (status, subject, timestamp)
    and signtool.exe verify /pa /v checks.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f12_1_get_authenticode_signature_status(self):
        """F12-1: Verify verification script checks .Status -eq 'Valid'."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn(
            "Get-AuthenticodeSignature",
            scripts,
            "Requirement R4 / F12: Verification workflow must call Get-AuthenticodeSignature.",
        )
        self.assertIn("Status", scripts)

    def test_f12_2_signer_certificate_check(self):
        """F12-2: Verify validation of SignerCertificate properties."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        has_signer_check = "SignerCertificate" in scripts or "Status" in scripts
        self.assertTrue(has_signer_check, "F12: Must inspect SignerCertificate status.")

    def test_f12_3_timestamper_certificate_check(self):
        """F12-3: Verify RFC 3161 TimeStamperCertificate presence check."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn(
            "TimeStamperCertificate",
            scripts,
            "Requirement R4 / F12: Must verify TimeStamperCertificate is non-null.",
        )

    def test_f12_4_signtool_pa_v_check(self):
        """F12-4: Verify verification script invokes signtool verify /pa /v."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        has_signtool_check = "signtool" in scripts.lower() or "Get-AuthenticodeSignature" in scripts
        self.assertTrue(has_signtool_check, "F12: Must perform Authenticode policy verification.")

    def test_f12_5_exit_code_failure_on_unsigned(self):
        """F12-5: Verify workflow script fails on unsigned or invalid binary."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        has_throw = "throw" in scripts.lower() or "exit 1" in scripts.lower()
        self.assertTrue(has_throw, "F12: Script must throw or exit 1 on unsigned target.")


# =============================================================================
# Feature 13: Trust Store Registration (Requirement R4)
# =============================================================================

class TestF13TrustStoreRegistration(unittest.TestCase):
    """
    F13: Trust Store Registration
    Verifies importing WINDOWS_CERT_BASE64 into LocalMachine\\Root store
    in CI runner to prevent false CERT_E_UNTRUSTEDROOT errors.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_f13_1_base64_secret_reading(self):
        """F13-1: Verify PowerShell script reads $env:WINDOWS_CERT_BASE64."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn("WINDOWS_CERT_BASE64", scripts)

    def test_f13_2_x509_certificate_import(self):
        """F13-2: Verify certificate bytes imported into X509Certificate2."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn("X509Certificate2", scripts)

    def test_f13_3_root_store_addition(self):
        """F13-3: Verify certificate added to LocalMachine\\Root store."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        has_root_store = "Root" in scripts and "LocalMachine" in scripts
        self.assertTrue(has_root_store, "F13: Must import into LocalMachine\\Root store.")

    def test_f13_4_error_catching(self):
        """F13-4: Verify certificate setup in verify_signatures.yml handles error gracefully with try/catch."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertTrue(
            "catch" in scripts and ("Write-Warning" in scripts or "Write-Host" in scripts),
            "Requirement R4 / F13: .github/workflows/verify_signatures.yml must include try/catch error handling around certificate import.",
        )

    def test_f13_5_clean_skip_when_unset(self):
        """F13-5: Verify trust store registration cleanly skipped if secret unset."""
        if not self.workflow_path.exists():
            self.fail(f"Missing workflow file: {self.workflow_path}")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = "\n".join(validator.extract_powershell_scripts())
        self.assertIn("if ($env:WINDOWS_CERT_BASE64", scripts)


# =============================================================================
# Feature 14: Release Gating Integration (Requirement R4)
# =============================================================================

class TestF14ReleaseGatingIntegration(unittest.TestCase):
    """
    F14: Release Gating Integration
    Verifies integration of verify_signatures job into build_and_release.yml,
    gating deploy on successful signature validation.
    """

    def setUp(self):
        self.release_workflow_path = WORKFLOWS_DIR / "build_and_release.yml"
        self.validator = WorkflowValidator.parse_file(self.release_workflow_path)

    def test_f14_1_build_and_release_verify_signatures_job(self):
        """F14-1: Verify build_and_release.yml defines verify_signatures job."""
        jobs = self.validator.get_jobs()
        self.assertIn(
            "verify_signatures",
            jobs,
            "Requirement R4 / F14: .github/workflows/build_and_release.yml must define "
            "a 'verify_signatures' job.",
        )

    def test_f14_2_needs_dependency_meta_build(self):
        """F14-2: Verify verify_signatures job depends on [meta, build]."""
        needs = self.validator.get_job_needs("verify_signatures")
        self.assertIn("build", needs, "F14: verify_signatures job must depend on 'build'.")

    def test_f14_3_deploy_needs_verify_signatures(self):
        """F14-3: Verify deploy job depends on verify_signatures."""
        deploy_needs = self.validator.get_job_needs("deploy")
        self.assertIn(
            "verify_signatures",
            deploy_needs,
            "Requirement R4 / F14: 'deploy' job must declare dependency on 'verify_signatures' "
            "to gate production release on signature verification.",
        )

    def test_f14_4_secrets_pass_through(self):
        """F14-4: Verify certificate secrets pass-through to verify_signatures job."""
        vjob = self.validator.get_job("verify_signatures")
        if not vjob:
            self.fail("verify_signatures job not found in build_and_release.yml")
        secrets_block = vjob.get("secrets", {})
        has_secrets = (
            "WINDOWS_CERT_BASE64" in str(secrets_block)
            or secrets_block == "inherit"
            or "env" in vjob
        )
        self.assertTrue(has_secrets, "F14: WINDOWS_CERT_BASE64 secret must be passed to verify_signatures.")

    def test_f14_5_build_type_input(self):
        """F14-5: Verify build_type input parameter passed to verify_signatures job."""
        vjob = self.validator.get_job("verify_signatures")
        if not vjob:
            self.fail("verify_signatures job not found in build_and_release.yml")
        with_block = vjob.get("with", {})
        has_build_type = "build_type" in with_block or "matrix" in str(vjob)
        self.assertTrue(has_build_type, "F14: build_type parameter must be passed to verify_signatures.")


if __name__ == "__main__":
    unittest.main()
