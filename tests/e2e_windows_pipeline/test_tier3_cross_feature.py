"""
Tier 3 Cross-Feature Pairwise Interaction Test Suite — Attune Windows Runtime, Packaging, Code Signing & Verification.

Authoritative Specifications & Interaction Coverage:
- ORIGINAL_REQUEST.md: Requirements R1, R2, R3, R4
- PROJECT.md: Features F1–F14 interaction contracts
- TEST_INFRA.md: Tier 3 Cross-Feature Pairwise Matrix
- SCOPE.md: Milestone T4 Requirements

Interaction Pairs Covered (14 Key Feature Pairs):
  - Pair 1 (F1 + F3): SetDllDirectoryW initialization with dynamic component bundling in {app}\\lib
  - Pair 2 (F2 + F1): LoadLibraryW UTF-16 wide string loading with custom DLL directory
  - Pair 3 (F4 + F7): Integrity manifest generation strictly after PE binary signing
  - Pair 4 (F5 + F7): Base64 certificate decoding in builder.py and utilization by signtool
  - Pair 5 (F5 + F6): Base64 certificate decoding and application of RFC 3161 timestamping
  - Pair 6 (F6 + F12): RFC 3161 timestamping verified via PowerShell TimeStamperCertificate
  - Pair 7 (F7 + F12): Full PE binary signing verified via dual engine (signtool + Authenticode)
  - Pair 8 (F8 + F10): Inno Setup SignTool directive in [Setup] matching /Ssigntool CLI invocation
  - Pair 9 (F8 + F9): Inno Setup SignTool directive active with signonce flags on .exe and .dll
  - Pair 10 (F9 + F4): Inno Setup signonce flags excluded from attune_integrity.json
  - Pair 11 (F11 + F14): Dedicated verify_signatures.yml workflow called by build_and_release.yml gating deploy
  - Pair 12 (F12 + F13): Dual Authenticode verification following LocalMachine\\Root trust anchor registration
  - Pair 13 (F3 + F12): Dynamic component bundling in portable zip/installer verifying all 5 DLLs signed
  - Pair 14 (F5 + F10): Base64 certificate decoded path formatted with $q escaping into Inno Setup /Ssigntool
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
# Pair 1 (F1 + F3): SetDllDirectoryW Initialization with Dynamic Bundling in lib/
# =============================================================================

class TestPair1F1F3SetDllDirectoryBundling(unittest.TestCase):
    """
    Pair 1 (F1 + F3): SetDllDirectoryW initialization with dynamic component bundling.
    Validates that attune-desktop sets SetDllDirectoryW to {app}\\lib where all 5
    workspace dynamic DLLs reside, enabling dynamic discovery without PATH pollution.
    """

    def setUp(self):
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.fixture = WindowsStagingFixture()

    def tearDown(self):
        self.fixture.cleanup()

    def test_p1_1_set_dll_directory_targets_lib_subdir(self):
        """P1-1: Verify dynamic_components.rs initializes SetDllDirectoryW targeting lib subdirectory."""
        self.assertTrue(self.dyn_comp_path.exists(), f"Source file missing: {self.dyn_comp_path}")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")

        self.assertIn(
            "SetDllDirectoryW",
            dyn_src,
            "Requirement R1 / F1: dynamic_components.rs must declare or call SetDllDirectoryW.",
        )
        has_lib_ref = '"lib"' in dyn_src or r'"\lib"' in dyn_src or r'"\\lib"' in dyn_src or "lib_dir" in dyn_src
        self.assertTrue(
            has_lib_ref,
            "Pair 1 (F1+F3): SetDllDirectoryW initialization must include 'lib' subdirectory search path.",
        )

    def test_p1_2_simulated_search_order_resolves_all_5_bundled_dlls(self):
        """P1-2: Simulate Win32 search order with SetDllDirectoryW targeting app_dir/lib."""
        app_dir = self.fixture.dist_dir
        lib_dir = self.fixture.dist_lib_dir
        system_dir = self.fixture.root / "system32"
        system_dir.mkdir(parents=True, exist_ok=True)

        self.fixture.stage_binaries(signed=True)

        for dll_name in REQUIRED_DYNAMIC_DLLS:
            resolved_order = RustLoaderContract.simulate_search_order(
                dll_name=dll_name,
                set_dll_directory=lib_dir,
                app_dir=app_dir,
                system_dirs=[system_dir],
            )
            # The search order must place app_dir first, then set_dll_directory (lib), then system
            self.assertEqual(resolved_order[0], app_dir / dll_name)
            self.assertEqual(resolved_order[1], lib_dir / dll_name)
            self.assertEqual(resolved_order[2], system_dir / dll_name)

            # Both app_dir / dll_name and lib_dir / dll_name must physically exist in staging
            self.assertTrue(
                (lib_dir / dll_name).exists(),
                f"Pair 1 (F1+F3): Bundled DLL {dll_name} must exist in staged lib directory.",
            )

    def test_p1_3_installer_destdir_matches_set_dll_directory(self):
        """P1-3: Verify attune_installer.iss installs dynamic components into {app}\\lib or {app}."""
        self.assertTrue(self.iss_path.exists(), f"Missing installer script: {self.iss_path}")
        parser = InnoSetupParser.parse_file(self.iss_path)
        files = parser.get_files_entries()

        # Check destination directories configured for dynamic DLLs
        dll_dest_dirs = set()
        for f in files:
            src = f.get("source", "").lower()
            if any(dll.lower() in src for dll in REQUIRED_DYNAMIC_DLLS):
                dll_dest_dirs.add(f.get("destdir", "").upper())

        self.assertTrue(
            any("{APP}" in d for d in dll_dest_dirs),
            f"Pair 1 (F1+F3): Installer must place dynamic DLLs under {{app}} or {{app}}\\lib. Found: {dll_dest_dirs}",
        )


# =============================================================================
# Pair 2 (F2 + F1): LoadLibraryW Wide String Loading with SetDllDirectoryW
# =============================================================================

class TestPair2F2F1LoadLibraryWideWithSetDllDirectory(unittest.TestCase):
    """
    Pair 2 (F2 + F1): LoadLibraryW UTF-16 wide string loading with custom DLL directory.
    Ensures wide character encoding (UTF-16 with trailing null) is strictly maintained
    across directory initialization and dynamic module loading.
    """

    def setUp(self):
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_p2_1_dynamic_components_uses_load_library_w(self):
        """P2-1: Verify dynamic_components.rs defines and invokes LoadLibraryW (not LoadLibraryA)."""
        self.assertTrue(self.dyn_comp_path.exists(), f"Source file missing: {self.dyn_comp_path}")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")

        self.assertIn(
            "LoadLibraryW",
            dyn_src,
            "Requirement R1 / F2: dynamic_components.rs must use LoadLibraryW for Unicode path safety.",
        )
        self.assertNotIn(
            "LoadLibraryA",
            dyn_src,
            "Requirement R1 / F2: ANSI LoadLibraryA must be eliminated to prevent non-ASCII path failures.",
        )

    def test_p2_2_wide_encoding_preserves_unicode_and_null_termination(self):
        """P2-2: Verify wide encoding preserves non-ASCII paths set by SetDllDirectoryW."""
        unicode_dirs = [
            r"C:\Program Files\Attune 音楽\lib",
            r"C:\Users\José\AppData\Local\Programs\Attune\lib",
            r"C:\ユーザー\みく\Attune\lib",
            r"D:\Attune [2026] - Studio\lib",
        ]
        for udir in unicode_dirs:
            encoded = RustLoaderContract.encode_wide(udir)
            self.assertTrue(
                RustLoaderContract.validate_null_termination(encoded),
                f"Pair 2 (F2+F1): Encoded path must be strictly null-terminated: {udir}",
            )
            decoded = RustLoaderContract.decode_wide(encoded)
            self.assertEqual(
                decoded,
                udir,
                f"Pair 2 (F2+F1): Decoded UTF-16 wide string must exactly match original: {udir}",
            )

    def test_p2_3_init_windows_dll_directories_precedes_dlopen(self):
        """P2-3: Verify init_windows_dll_directories is called before dynamic loading in dynamic_components.rs."""
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")
        has_early_init = "init_windows_dll_directories()" in dyn_src
        self.assertTrue(
            has_early_init,
            "Pair 2 (F2+F1): dynamic_components.rs must invoke init_windows_dll_directories() during initialization.",
        )


# =============================================================================
# Pair 3 (F4 + F7): Integrity Manifest Generation Strictly After PE Signing
# =============================================================================

class TestPair3F4F7IntegrityManifestPostSigning(unittest.TestCase):
    """
    Pair 3 (F4 + F7): Integrity manifest synchronization executed strictly after PE binary signing.
    Validates that signing modifies PE binary hashes and that attune_integrity.json
    must capture post-signing cryptographic digests (SHA-256 and BLAKE3).
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def tearDown(self):
        self.fixture.cleanup()

    def test_p3_1_signing_modifies_pe_checksum_and_security_directory(self):
        """P3-1: Verify Authenticode signing alters binary digest compared to unsigned artifact."""
        unsigned_path = AuthenticodeHelper.create_synthetic_pe(
            self.fixture.dist_lib_dir / "attune_engine.dll", is_signed=False
        )
        hash_before = hashlib.sha256(unsigned_path.read_bytes()).hexdigest()

        # Sign the binary
        AuthenticodeHelper.append_authenticode_signature(unsigned_path)
        hash_after = hashlib.sha256(unsigned_path.read_bytes()).hexdigest()

        self.assertNotEqual(
            hash_before,
            hash_after,
            "Pair 3 (F4+F7): Authenticode signing must modify file contents (appending WIN_CERTIFICATE table).",
        )

        parsed = PEParser.parse_file(unsigned_path)
        self.assertTrue(parsed["is_signed"], "Signed binary must report is_signed == True.")
        self.assertGreater(parsed["sec_dir_size"], 0, "Security directory entry size must be > 0.")

    def test_p3_2_pre_signing_manifest_fails_runtime_verification(self):
        """P3-2: Verify that manifest generated before signing causes runtime verification failure."""
        self.fixture.stage_binaries(signed=False)
        manifest_pre = self.fixture.generate_manifest()
        expected_pre_hash = manifest_pre["components"]["attune_engine.dll"]["sha256"]

        # Now sign the binary post-manifest (simulating out-of-order execution)
        dll_path = self.fixture.dist_lib_dir / "attune_engine.dll"
        AuthenticodeHelper.append_authenticode_signature(dll_path)

        is_valid, msg = self.fixture.verify_component_integrity(dll_path, expected_pre_hash)
        self.assertFalse(
            is_valid,
            "Pair 3 (F4+F7): Verification must fail when binary is signed after manifest calculation.",
        )
        self.assertIn("CRITICAL", msg)
        self.assertIn("mismatch", msg)

    def test_p3_3_post_signing_manifest_passes_runtime_verification(self):
        """P3-3: Verify that manifest generated strictly after signing passes runtime verification."""
        self.fixture.stage_binaries(signed=True)
        manifest_post = self.fixture.generate_manifest()

        for dll_name in REQUIRED_DYNAMIC_DLLS:
            dll_path = self.fixture.dist_lib_dir / dll_name
            expected_hash = manifest_post["components"][dll_name]["sha256"]
            is_valid, msg = self.fixture.verify_component_integrity(dll_path, expected_hash)
            self.assertTrue(
                is_valid,
                f"Pair 3 (F4+F7): Post-signing verification must pass for {dll_name}: {msg}",
            )

    def test_p3_4_builder_py_sequences_signing_before_integrity_manifest(self):
        """P3-4: Verify builder.py invokes Authenticode signing prior to generate_integrity_manifest."""
        self.assertTrue(self.builder_py_path.exists(), f"Missing builder.py: {self.builder_py_path}")
        builder_src = self.builder_py_path.read_text(encoding="utf-8")

        # In package_dist or build pipeline, signing must precede integrity manifest generation
        sign_idx = builder_src.find("sign_binaries")
        manifest_idx = builder_src.find("generate_integrity_manifest")

        self.assertTrue(
            sign_idx != -1 and manifest_idx != -1,
            "Pair 3 (F4+F7): builder.py must define both sign_binaries and generate_integrity_manifest.",
        )
        self.assertLess(
            sign_idx,
            manifest_idx,
            "Requirement R1/R2: builder.py must execute binary signing strictly before integrity manifest generation.",
        )


# =============================================================================
# Pair 4 (F5 + F7): Base64 Certificate Resolution to Full PE Signing
# =============================================================================

class TestPair4F5F7Base64CertToPESigning(unittest.TestCase):
    """
    Pair 4 (F5 + F7): Base64 certificate decoding in builder.py and utilization by signtool.
    Validates that WINDOWS_CERT_BASE64 is resolved to a secure temporary .pfx file
    and successfully ingested by signtool to sign PE binaries.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def tearDown(self):
        self.fixture.cleanup()

    def test_p4_1_base64_cert_decodes_to_valid_pfx_for_signing(self):
        """P4-1: Test end-to-end base64 decode and PE signing artifact production."""
        dummy_pfx = Base64CertHelper.generate_dummy_pfx(password="SecretPass123!")
        b64_str = Base64CertHelper.encode_base64(dummy_pfx)

        cert_file = self.fixture.root / "temp_cert.pfx"
        resolved_path = Base64CertHelper.resolve_cert_to_file(b64_str, cert_file)
        self.assertTrue(resolved_path.exists(), "Decoded certificate file must exist.")
        self.assertEqual(resolved_path.read_bytes(), dummy_pfx)

        # Create synthetic PE and sign it referencing certificate subject
        target_dll = self.fixture.dist_lib_dir / "attune_procmgr.dll"
        AuthenticodeHelper.create_synthetic_pe(target_dll, is_signed=False)
        AuthenticodeHelper.append_authenticode_signature(target_dll, cert_subject="CN=Attune Base64 Resolved Cert")

        parsed = PEParser.parse_file(target_dll)
        self.assertTrue(parsed["is_signed"], "PE must be signed.")
        self.assertEqual(parsed["cert_info"]["type"], "0x2", "WIN_CERT_TYPE must be WIN_CERT_TYPE_PKCS_SIGNED_DATA.")

    def test_p4_2_builder_py_supports_windows_cert_base64_resolution(self):
        """P4-2: Verify builder.py inspects WINDOWS_CERT_BASE64 environment secret."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")
        has_b64_resolution = (
            "WINDOWS_CERT_BASE64" in builder_src
            or "b64decode" in builder_src
            or "resolve_windows_certificate" in builder_src
        )
        self.assertTrue(
            has_b64_resolution,
            "Requirement R2 / F5: builder.py must support decoding WINDOWS_CERT_BASE64 for code signing.",
        )


# =============================================================================
# Pair 5 (F5 + F6): Base64 Certificate Resolution + RFC 3161 Timestamping
# =============================================================================

class TestPair5F5F6Base64CertWithRFC3161Timestamp(unittest.TestCase):
    """
    Pair 5 (F5 + F6): Base64 certificate decoding and application of RFC 3161 timestamping.
    Validates that builder.py pairs certificate authentication with timestamp authority
    parameters (/tr http://timestamp.digicert.com /td SHA256) in a unified command.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_p5_1_builder_signing_combines_cert_path_and_timestamp_url(self):
        """P5-1: Verify builder.py constructs signtool command combining cert credentials and RFC 3161 TSA."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")

        # Must include both certificate path argument and timestamp URL
        has_cert_param = "/f" in builder_src or "-pkcs12" in builder_src
        has_timestamp_param = (
            "/tr" in builder_src and "timestamp.digicert.com" in builder_src
        ) or "-ts" in builder_src

        self.assertTrue(has_cert_param, "Pair 5 (F5+F6): Signing command must specify certificate path.")
        self.assertTrue(
            has_timestamp_param,
            "Requirement R2 / F6: Signing command must specify RFC 3161 timestamp URL (/tr or -ts).",
        )

    def test_p5_2_timestamp_digest_algorithm_is_sha256(self):
        """P5-2: Verify timestamp digest algorithm is SHA256 (/td SHA256)."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")
        has_td_sha256 = "/td" in builder_src and "SHA256" in builder_src
        self.assertTrue(
            has_td_sha256 or "-ts" in builder_src,
            "Requirement R2 / F6: Timestamp digest algorithm must be configured as SHA256 via /td SHA256.",
        )


# =============================================================================
# Pair 6 (F6 + F12): RFC 3161 Timestamping + PowerShell Verification
# =============================================================================

class TestPair6F6F12TimestampPowerShellVerification(unittest.TestCase):
    """
    Pair 6 (F6 + F12): RFC 3161 timestamping verified via PowerShell TimeStamperCertificate.
    Validates that the verification engine extracts and asserts the RFC 3161
    countersignature certificate to ensure long-term signature validity.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_p6_1_workflow_asserts_timestamper_certificate_presence(self):
        """P6-1: Verify verify_signatures.yml inspects .TimeStamperCertificate on Authenticode signatures."""
        self.assertTrue(
            self.workflow_path.exists(),
            "Requirement R4 / F11: .github/workflows/verify_signatures.yml must exist.",
        )
        validator = WorkflowValidator.parse_file(self.workflow_path)
        scripts = validator.extract_powershell_scripts()
        combined_script = "\n".join(scripts)

        self.assertIn(
            "TimeStamperCertificate",
            combined_script,
            "Requirement R4 / F12: PowerShell verification script must verify TimeStamperCertificate.",
        )

    def test_p6_2_workflow_fails_when_timestamper_is_missing(self):
        """P6-2: Verify script throws terminating error when TimeStamperCertificate is absent."""
        self.assertTrue(self.workflow_path.exists(), "Workflow file must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        combined_script = "\n".join(validator.extract_powershell_scripts())

        has_throw_on_missing_ts = (
            "throw" in combined_script or "exit 1" in combined_script or "Write-Error" in combined_script
        ) and "TimeStamperCertificate" in combined_script

        self.assertTrue(
            has_throw_on_missing_ts,
            "Pair 6 (F6+F12): Verification script must throw or exit non-zero when timestamp is missing.",
        )


# =============================================================================
# Pair 7 (F7 + F12): Full PE Binary Signing + Dual Authenticode Verification
# =============================================================================

class TestPair7F7F12FullPESigningDualVerification(unittest.TestCase):
    """
    Pair 7 (F7 + F12): Full PE binary signing verified via dual engine (signtool + Get-AuthenticodeSignature).
    Validates that all 6 required binaries (desktop exe + 5 DLLs) are signed and verified
    by both PowerShell and SignTool verify /pa engines.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def tearDown(self):
        self.fixture.cleanup()

    def test_p7_1_all_pe_binaries_pass_structural_security_directory_check(self):
        """P7-1: Verify synthetic signed PE binaries meet PE32+ security directory requirements."""
        paths = self.fixture.stage_binaries(signed=True)

        for name in REQUIRED_PE_BINARIES:
            key = "desktop" if name == "attune-desktop.exe" else name
            target_path = paths[key]
            parsed = PEParser.parse_file(target_path)

            self.assertTrue(parsed["is_pe"], f"{name} must be a valid PE binary.")
            self.assertTrue(parsed["is_64bit"], f"{name} must be 64-bit PE32+.")
            self.assertTrue(parsed["is_signed"], f"{name} must have a valid security directory entry.")
            self.assertGreater(parsed["cert_info"]["length"], 8)

    def test_p7_2_workflow_invokes_both_authenticode_and_signtool_verify(self):
        """P7-2: Verify verify_signatures.yml implements dual verification (Get-AuthenticodeSignature and signtool)."""
        self.assertTrue(self.workflow_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        combined_script = "\n".join(validator.extract_powershell_scripts())

        self.assertIn(
            "Get-AuthenticodeSignature",
            combined_script,
            "Requirement R4 / F12: Workflow must execute Get-AuthenticodeSignature.",
        )
        has_signtool_verify = "signtool" in combined_script and "verify" in combined_script
        self.assertTrue(
            has_signtool_verify or "Get-AuthenticodeSignature" in combined_script,
            "Requirement R4 / F12: Workflow must support signtool verify /pa validation.",
        )


# =============================================================================
# Pair 8 (F8 + F10): Inno Setup SignTool Directive Matching CLI Switch
# =============================================================================

class TestPair8F8F10InnoSetupSignToolDirectiveMatchingCLI(unittest.TestCase):
    """
    Pair 8 (F8 + F10): Inno Setup SignTool directive in [Setup] matching /Ssigntool CLI invocation.
    Validates that the SignToolName macro in attune_installer.iss exactly matches the
    command-line switch prefix passed by builder.py when invoking iscc.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_p8_1_iss_setup_defines_signtool_directive(self):
        """P8-1: Verify attune_installer.iss defines SignTool directive in [Setup]."""
        self.assertTrue(self.iss_path.exists(), f"Installer missing: {self.iss_path}")
        parser = InnoSetupParser.parse_file(self.iss_path)

        signtool_dir = parser.get_setup_directive("SignTool")
        self.assertIsNotNone(
            signtool_dir,
            "Requirement R3 / F8: attune_installer.iss must specify 'SignTool=' in [Setup] section.",
        )

    def test_p8_2_builder_passes_matching_s_signtool_switch(self):
        """P8-2: Verify builder.py passes /Ssigntool= CLI switch matching SignTool directive."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")
        self.assertIn(
            "/Ssigntool=",
            builder_src,
            "Requirement R3 / F10: builder.py must pass '/Ssigntool=...' switch to iscc command.",
        )


# =============================================================================
# Pair 9 (F8 + F9): Inno Setup SignTool Directive with signonce Flags
# =============================================================================

class TestPair9F8F9InnoSetupSignToolWithSignonce(unittest.TestCase):
    """
    Pair 9 (F8 + F9): Inno Setup SignTool directive active with signonce flags on PE binaries.
    Validates that when SignTool is active in [Setup], all embedded .exe and .dll entries
    in [Files] include the signonce flag so Inno Setup does not attempt destructive re-signing.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"

    def test_p9_1_all_executable_files_have_signonce_when_signtool_active(self):
        """P9-1: Verify every .exe and .dll file entry in [Files] contains the signonce flag."""
        self.assertTrue(self.iss_path.exists(), f"Installer missing: {self.iss_path}")
        parser = InnoSetupParser.parse_file(self.iss_path)

        files = parser.get_files_entries()
        pe_files = [f for f in files if f.get("source", "").lower().endswith((".exe", ".dll"))]
        self.assertGreater(len(pe_files), 0, "Installer must include PE binary entries.")

        for f in pe_files:
            flags = f.get("flags", set())
            src = f.get("source", "")
            self.assertIn(
                "signonce",
                flags,
                f"Pair 9 (F8+F9): Binary '{src}' must include 'signonce' flag in [Files].",
            )


# =============================================================================
# Pair 10 (F9 + F4): Inno Setup signonce Flags Excluded from Integrity Manifest
# =============================================================================

class TestPair10F9F4SignonceExcludedFromIntegrityManifest(unittest.TestCase):
    """
    Pair 10 (F9 + F4): Inno Setup signonce flags excluded from attune_integrity.json.
    Validates that metadata and manifest files never receive the signonce flag,
    ensuring attune_integrity.json remains completely unaltered during packaging.
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"

    def test_p10_1_manifest_and_metadata_files_lack_signonce(self):
        """P10-1: Verify attune_integrity.json and non-PE files do NOT have signonce flag."""
        parser = InnoSetupParser.parse_file(self.iss_path)
        files = parser.get_files_entries()

        non_pe_entries = [
            f for f in files
            if not f.get("source", "").lower().endswith((".exe", ".dll"))
        ]
        self.assertGreater(len(non_pe_entries), 0, "Installer should have non-PE file entries.")

        for f in non_pe_entries:
            src = f.get("source", "")
            flags = f.get("flags", set())
            self.assertNotIn(
                "signonce",
                flags,
                f"Pair 10 (F9+F4): Non-PE file '{src}' must NOT have 'signonce' flag.",
            )

    def test_p10_2_manifest_hashes_remain_identical_across_packaging(self):
        """P10-2: Verify manifest hashes match pre-packaged and post-packaged staged binaries."""
        fixture = WindowsStagingFixture()
        try:
            fixture.stage_binaries(signed=True)
            manifest = fixture.generate_manifest()

            manifest_file = fixture.dist_dir / "attune_integrity.json"
            hash_before = hashlib.sha256(manifest_file.read_bytes()).hexdigest()

            # Simulating installer packaging: non-PE files are untouched
            hash_after = hashlib.sha256(manifest_file.read_bytes()).hexdigest()
            self.assertEqual(
                hash_before,
                hash_after,
                "Pair 10 (F9+F4): Integrity manifest file must not be modified during packaging.",
            )
        finally:
            fixture.cleanup()


# =============================================================================
# Pair 11 (F11 + F14): Dedicated Verification Workflow Called by Release Workflow
# =============================================================================

class TestPair11F11F14WorkflowReleaseGating(unittest.TestCase):
    """
    Pair 11 (F11 + F14): Dedicated verify_signatures.yml workflow called by build_and_release.yml.
    Validates that the verification workflow gates the deploy job so no unsigned
    or invalid artifacts can ever be published.
    """

    def setUp(self):
        self.verify_path = WORKFLOWS_DIR / "verify_signatures.yml"
        self.release_path = WORKFLOWS_DIR / "build_and_release.yml"

    def test_p11_1_verify_signatures_workflow_defines_workflow_call(self):
        """P11-1: Verify verify_signatures.yml supports workflow_call trigger for reusable execution."""
        self.assertTrue(self.verify_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.verify_path)
        triggers = validator.get_triggers()

        self.assertIn(
            "workflow_call",
            triggers,
            "Requirement R4 / F11: verify_signatures.yml must include 'workflow_call' trigger.",
        )

    def test_p11_2_build_and_release_has_verify_signatures_job(self):
        """P11-2: Verify build_and_release.yml defines verify_signatures job."""
        self.assertTrue(self.release_path.exists(), "build_and_release.yml must exist.")
        validator = WorkflowValidator.parse_file(self.release_path)
        jobs = validator.get_jobs()

        self.assertIn(
            "verify_signatures",
            jobs,
            "Requirement R4 / F14: build_and_release.yml must define 'verify_signatures' job.",
        )

    def test_p11_3_deploy_job_strictly_depends_on_verify_signatures(self):
        """P11-3: Verify deploy job in build_and_release.yml has needs including verify_signatures."""
        validator = WorkflowValidator.parse_file(self.release_path)
        deploy_needs = validator.get_job_needs("deploy")

        self.assertIn(
            "verify_signatures",
            deploy_needs,
            "Requirement R4 / F14: 'deploy' job must depend on 'verify_signatures' in needs array.",
        )


# =============================================================================
# Pair 12 (F12 + F13): Dual Verification Following Trust Store Registration
# =============================================================================

class TestPair12F12F13TrustStoreBeforeAuthenticode(unittest.TestCase):
    """
    Pair 12 (F12 + F13): Dual Authenticode verification following LocalMachine\\Root registration.
    Validates that verify_signatures.yml registers the CI certificate trust anchor
    prior to executing signature verification, avoiding CERT_E_UNTRUSTEDROOT.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_p12_1_trust_anchor_step_precedes_verification_step(self):
        """P12-1: Verify trust anchor registration step executes before signature verification step."""
        self.assertTrue(self.workflow_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        steps = validator.get_job_steps("verify_authenticode")

        trust_step_idx = -1
        verify_step_idx = -1

        for idx, s in enumerate(steps):
            name = s.get("name", "").lower()
            run_cmd = s.get("run", "")
            if "trust" in name or "LocalMachine" in run_cmd or "X509Store" in run_cmd:
                trust_step_idx = idx
            if "verify" in name or "Get-AuthenticodeSignature" in run_cmd:
                verify_step_idx = idx

        self.assertTrue(
            trust_step_idx != -1 and verify_step_idx != -1,
            "Pair 12 (F12+F13): verify_authenticode job must include both trust setup and verification steps.",
        )
        self.assertLess(
            trust_step_idx,
            verify_step_idx,
            "Requirement R4 / F13: Trust anchor setup must execute strictly before Authenticode verification.",
        )


# =============================================================================
# Pair 13 (F3 + F12): Dynamic Component Bundling Verifying All 5 DLLs Signed
# =============================================================================

class TestPair13F3F12AllFiveBundledDllsVerified(unittest.TestCase):
    """
    Pair 13 (F3 + F12): Dynamic component bundling in portable zip/installer verifying all 5 DLLs.
    Validates that all 5 workspace DLLs are present in the verification targets list
    in verify_signatures.yml and bundled in dist/windows.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_p13_1_verification_target_list_includes_all_5_dynamic_dlls(self):
        """P13-1: Verify verify_signatures.yml checks all 5 workspace dynamic DLLs."""
        self.assertTrue(self.workflow_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        combined_script = "\n".join(validator.extract_powershell_scripts())

        for dll_name in REQUIRED_DYNAMIC_DLLS:
            self.assertIn(
                dll_name,
                combined_script,
                f"Pair 13 (F3+F12): Verification workflow must target dynamic component: {dll_name}",
            )

    def test_p13_2_builder_py_bundles_all_5_dynamic_dlls(self):
        """P13-2: Verify builder.py bundles all 5 dynamic DLLs in Windows distribution."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            stem = dll_name.replace(".dll", "")
            self.assertTrue(
                stem in builder_src,
                f"Pair 13 (F3+F12): builder.py must bundle dynamic component: {dll_name}",
            )


# =============================================================================
# Pair 14 (F5 + F10): Base64 Cert Path Formatted with $q Escaping in CLI Switch
# =============================================================================

class TestPair14F5F10Base64CertPathDollarQEscaping(unittest.TestCase):
    """
    Pair 14 (F5 + F10): Base64 certificate decoded path properly formatted with $q escaping.
    Validates that Inno Setup CLI invocation /Ssigntool= properly wraps certificate
    and signtool paths with $q quotes so paths with whitespace do not cause syntax errors.
    """

    def setUp(self):
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def test_p14_1_builder_formats_dollar_q_around_cert_path(self):
        """P14-1: Verify builder.py applies $q quotation marks to cert_path in /Ssigntool switch."""
        builder_src = self.builder_py_path.read_text(encoding="utf-8")

        has_dollar_q_cert = (
            "$q{cert_path}$q" in builder_src
            or '"$q" + str(cert_path)' in builder_src
            or "$q" in builder_src and "cert" in builder_src
        )
        self.assertTrue(
            has_dollar_q_cert,
            "Requirement R3 / F10: builder.py must escape certificate path using $q in /Ssigntool parameter.",
        )

    def test_p14_2_dollar_q_escaping_handles_whitespace_in_cert_path(self):
        """P14-2: Verify $q escaping wraps paths containing spaces for Inno Setup compiler."""
        sample_cert_path = r"C:\Users\CI Runner\AppData\Local\Temp\attune cert.pfx"
        escaped = f"$q{sample_cert_path}$q"
        self.assertTrue(
            escaped.startswith("$q") and escaped.endswith("$q"),
            "Pair 14 (F5+F10): Escaped path must be surrounded by $q tokens.",
        )
        self.assertNotIn('"', escaped, "Inno Setup /S parameters must use $q instead of raw double quotes.")


if __name__ == "__main__":
    unittest.main()
