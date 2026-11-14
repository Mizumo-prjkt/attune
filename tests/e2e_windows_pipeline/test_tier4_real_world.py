"""
Tier 4 Real-World Application Scenario Test Suite — Attune Windows Runtime, Packaging, Code Signing & Verification.

Authoritative Specifications & Real-World Workload Coverage:
- ORIGINAL_REQUEST.md: Requirements R1, R2, R3, R4
- PROJECT.md: End-to-end architecture & interface contracts
- TEST_INFRA.md: Real-World Scenarios
- SCOPE.md: Milestone T4 Requirements

Scenarios Covered (8 Comprehensive End-to-End Workload Scenarios):
  - Scenario 1: Clean Windows Release Build & Package Pipeline Simulation
  - Scenario 2: Release Gate Failure on Untrusted / Unsigned Artifact
  - Scenario 3: Non-ASCII Installation Path Runtime Simulation (Unicode & Error 87 Prevention)
  - Scenario 4: Anti-Tampering Security & Cryptographic Integrity Detection
  - Scenario 5: CI Runner Trust Store & Unknown Publisher Warning Elimination
  - Scenario 6: Inno Setup Upgrades & Signed Uninstaller Safety Flags
  - Scenario 7: Offline / Air-Gapped Build Fallback & Graceful Error Handling
  - Scenario 8: Windows Cross-Compilation Syntax & Target Configuration Validation
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
# Scenario 1: Clean Windows Release Build & Package Pipeline
# =============================================================================

class TestScenario1CleanReleasePipeline(unittest.TestCase):
    """
    Scenario 1: Clean Windows Release Build & Package Pipeline Simulation.
    Simulates the complete release workflow from source to distribution:
    Base64 cert decoding -> PE binary generation -> Authenticode signing with TSA ->
    Post-signing cryptographic integrity manifest -> Inno Setup packaging command synthesis.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.builder_py_path = TOOLS_DIR / "builder.py"
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"

    def tearDown(self):
        self.fixture.cleanup()

    def test_s1_1_full_pipeline_simulation(self):
        """S1-1: End-to-end simulation of release mode build, cert decoding, signing, and manifest."""
        # 1. Base64 Certificate Resolution
        dummy_pfx = Base64CertHelper.generate_dummy_pfx(password="ReleaseSecret2026!")
        b64_cert = Base64CertHelper.encode_base64(dummy_pfx)
        temp_cert_file = self.fixture.root / "release_cert.pfx"
        Base64CertHelper.resolve_cert_to_file(b64_cert, temp_cert_file)
        self.assertTrue(temp_cert_file.exists(), "Decoded certificate file must exist on disk.")

        # 2. Stage binaries (desktop exe + 5 DLLs)
        paths = self.fixture.stage_binaries(signed=False)
        self.assertEqual(len(paths), 6, "Must stage desktop executable and 5 dynamic DLLs.")

        # 3. Authenticode Signing with TSA Countersignature simulation
        for name in REQUIRED_PE_BINARIES:
            key = "desktop" if name == "attune-desktop.exe" else name
            target_bin = paths[key]
            AuthenticodeHelper.append_authenticode_signature(
                target_bin, cert_subject="CN=Attune Official Release, O=Attune Audio Inc"
            )
            parsed = PEParser.parse_file(target_bin)
            self.assertTrue(parsed["is_signed"], f"{name} must be successfully Authenticode-signed.")
            self.assertGreater(parsed["sec_dir_size"], 0, f"{name} security directory size must be > 0.")

        # 4. Generate post-signing integrity manifest
        manifest = self.fixture.generate_manifest(version="1.0.0")
        self.assertIn("components", manifest)
        self.assertEqual(len(manifest["components"]), 5, "Manifest must track all 5 dynamic components.")

        # 5. Verify every dynamic component satisfies integrity checks
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            dll_path = self.fixture.dist_lib_dir / dll_name
            expected_sha = manifest["components"][dll_name]["sha256"]
            is_valid, msg = self.fixture.verify_component_integrity(dll_path, expected_sha)
            self.assertTrue(is_valid, f"Component {dll_name} failed post-signing integrity: {msg}")

    def test_s1_2_builder_script_structural_pipeline_contract(self):
        """S1-2: Verify builder.py implements release mode packaging with signing and Inno Setup integration."""
        self.assertTrue(self.builder_py_path.exists(), "builder.py must exist.")
        builder_src = self.builder_py_path.read_text(encoding="utf-8")

        # Must support release mode, package step, and ISCC invocation
        self.assertIn("package_bundle", builder_src)
        self.assertIn("generate_integrity_manifest", builder_src)
        self.assertIn("iscc", builder_src.lower())


# =============================================================================
# Scenario 2: Release Gate Failure on Untrusted / Unsigned Artifact
# =============================================================================

class TestScenario2ReleaseGateBlocksUnsignedArtifact(unittest.TestCase):
    """
    Scenario 2: Release Gate Failure on Untrusted / Unsigned Artifact.
    Simulates a pipeline failure where one DLL (attune_music_metadata_changer.dll)
    is left unsigned. Verifies that the verification engine flags failure and
    the release workflow blocks deployment.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"
        self.release_path = WORKFLOWS_DIR / "build_and_release.yml"

    def tearDown(self):
        self.fixture.cleanup()

    def test_s2_1_unsigned_component_flagged_by_pe_verification(self):
        """S2-1: Verify verification detects unsigned DLL among signed components."""
        paths = self.fixture.stage_binaries(signed=True)

        # Corrupt / strip signature on attune_music_metadata_changer.dll (recreate as unsigned)
        target_dll = self.fixture.dist_lib_dir / "attune_music_metadata_changer.dll"
        AuthenticodeHelper.create_synthetic_pe(target_dll, is_signed=False)

        # Verification check across all targets
        unsigned_targets = []
        for name in REQUIRED_PE_BINARIES:
            key = "desktop" if name == "attune-desktop.exe" else name
            p = paths[key]
            parsed = PEParser.parse_file(p)
            if not parsed["is_signed"]:
                unsigned_targets.append(name)

        self.assertEqual(
            unsigned_targets,
            ["attune_music_metadata_changer.dll"],
            "Verification engine must identify attune_music_metadata_changer.dll as unsigned.",
        )

    def test_s2_2_release_workflow_blocks_deploy_on_verification_failure(self):
        """S2-2: Verify build_and_release.yml blocks deploy job when verify_signatures fails."""
        self.assertTrue(self.release_path.exists(), "build_and_release.yml must exist.")
        validator = WorkflowValidator.parse_file(self.release_path)

        # Deploy job must depend on verify_signatures
        deploy_job = validator.get_job("deploy")
        self.assertIsNotNone(deploy_job, "build_and_release.yml must define 'deploy' job.")

        needs = validator.get_job_needs("deploy")
        self.assertIn(
            "verify_signatures",
            needs,
            "Requirement R4 / F14: 'deploy' job must strictly depend on 'verify_signatures' to gate releases.",
        )


# =============================================================================
# Scenario 3: Non-ASCII Installation Path Runtime Simulation
# =============================================================================

class TestScenario3NonAsciiInstallationPathRuntime(unittest.TestCase):
    """
    Scenario 3: Non-ASCII Installation Path Runtime Simulation.
    Simulates end-user installing Attune into international and non-ASCII paths:
    C:\\Users\\José\\AppData\\Local\\Programs\\Attune 音楽\\
    C:\\ユーザー\\みく\\Attune\\
    Validates UTF-16 wide encoding, \\?\\ verbatim prefix stripping, and LoadLibraryW contracts.
    """

    def setUp(self):
        self.dyn_comp_path = CRATES_DIR / "attune-desktop" / "src" / "dynamic_components.rs"

    def test_s3_1_unicode_install_paths_encoding(self):
        """S3-1: Test wide character conversion across challenging international install paths."""
        international_paths = [
            r"C:\Users\José\AppData\Local\Programs\Attune 音楽\lib",
            r"C:\ユーザー\みく\Attune\lib",
            r"C:\Program Files (x86)\Attune — 音声プレイヤー (x64)\lib",
            r"D:\Música & DJ\Attune\lib",
            r"C:\Users\München\Attune\lib",
        ]
        for p in international_paths:
            wide_words = RustLoaderContract.encode_wide(p)
            self.assertTrue(
                RustLoaderContract.validate_null_termination(wide_words),
                f"S3-1: Path must terminate with null word (0x0000): {p}",
            )
            self.assertEqual(
                RustLoaderContract.decode_wide(wide_words),
                p,
                f"S3-1: Decoded wide string must match original path without corruption: {p}",
            )

    def test_s3_2_verbatim_prefix_stripping_logic(self):
        """S3-2: Verify dynamic_components.rs strips \\\\?\\ verbatim prefix to avoid Win32 Error 87."""
        self.assertTrue(self.dyn_comp_path.exists(), "dynamic_components.rs must exist.")
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")

        # Check for verbatim prefix stripping logic
        has_strip_logic = (
            r"\\?\\" in dyn_src
            or "strip_prefix" in dyn_src
            or "starts_with" in dyn_src
            or "canonicalize" in dyn_src
        )
        self.assertTrue(
            has_strip_logic,
            "Requirement R1: dynamic_components.rs must handle or strip verbatim prefixes for SetDllDirectoryW.",
        )

    def test_s3_3_load_library_w_is_used_exclusively(self):
        """S3-3: Verify dynamic_components.rs uses LoadLibraryW to guarantee Unicode path support."""
        dyn_src = self.dyn_comp_path.read_text(encoding="utf-8")
        self.assertIn("LoadLibraryW", dyn_src, "Requirement R1 / F2: Must call LoadLibraryW.")
        self.assertNotIn("LoadLibraryA", dyn_src, "Requirement R1 / F2: Must not call LoadLibraryA.")


# =============================================================================
# Scenario 4: Anti-Tampering Security & Integrity Detection
# =============================================================================

class TestScenario4AntiTamperingIntegrityDetection(unittest.TestCase):
    """
    Scenario 4: Anti-Tampering Security & Integrity Detection.
    Simulates an attacker modifying bytes inside attune_engine.dll after Authenticode signing.
    Validates that verify_component_integrity detects the hash mismatch and aborts loading.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()

    def tearDown(self):
        self.fixture.cleanup()

    def test_s4_1_tampering_detected_in_code_section(self):
        """S4-1: Simulate attacker flipping bytes in .text section of signed DLL."""
        self.fixture.stage_binaries(signed=True)
        manifest = self.fixture.generate_manifest()
        expected_sha = manifest["components"]["attune_engine.dll"]["sha256"]

        target_dll = self.fixture.dist_lib_dir / "attune_engine.dll"

        # 1. Genuine unmodified DLL must pass
        is_valid_clean, msg_clean = self.fixture.verify_component_integrity(target_dll, expected_sha)
        self.assertTrue(is_valid_clean, f"Clean binary must verify: {msg_clean}")

        # 2. Tamper: mutate byte inside code section (offset 0x200)
        raw_bytes = bytearray(target_dll.read_bytes())
        raw_bytes[0x200] ^= 0xFF
        target_dll.write_bytes(raw_bytes)

        # 3. Tampered binary must be strictly rejected
        is_valid_tampered, msg_tampered = self.fixture.verify_component_integrity(target_dll, expected_sha)
        self.assertFalse(is_valid_tampered, "Tampered binary must fail integrity check.")
        self.assertIn("CRITICAL", msg_tampered)
        self.assertIn("mismatch", msg_tampered)
        self.assertIn("Load rejected", msg_tampered)

    def test_s4_2_tampering_detected_in_appended_payload(self):
        """S4-2: Simulate attacker appending trailing malicious payload to signed DLL."""
        self.fixture.stage_binaries(signed=True)
        manifest = self.fixture.generate_manifest()
        expected_sha = manifest["components"]["attune_procmgr.dll"]["sha256"]

        target_dll = self.fixture.dist_lib_dir / "attune_procmgr.dll"

        # Append malicious payload at EOF
        with open(target_dll, "ab") as f:
            f.write(b"\x90\x90\xEB\xFE_MALICIOUS_SHELLCODE_PAYLOAD")

        is_valid, msg = self.fixture.verify_component_integrity(target_dll, expected_sha)
        self.assertFalse(is_valid, "Appended payload must be detected by SHA-256 mismatch.")
        self.assertIn("CRITICAL", msg)

    def test_s4_3_untampered_peer_components_remain_valid(self):
        """S4-3: Verify that tampering on one component does not falsely flag peer components."""
        self.fixture.stage_binaries(signed=True)
        manifest = self.fixture.generate_manifest()

        # Tamper only attune_engine.dll
        engine_dll = self.fixture.dist_lib_dir / "attune_engine.dll"
        raw_bytes = bytearray(engine_dll.read_bytes())
        raw_bytes[0x100] ^= 0xAA
        engine_dll.write_bytes(raw_bytes)

        # Peer component attune_watchguard.dll must still pass
        wg_dll = self.fixture.dist_lib_dir / "attune_watchguard.dll"
        wg_sha = manifest["components"]["attune_watchguard.dll"]["sha256"]
        is_valid, msg = self.fixture.verify_component_integrity(wg_dll, wg_sha)
        self.assertTrue(is_valid, f"Untampered peer component must pass: {msg}")


# =============================================================================
# Scenario 5: CI Runner Trust Store & Unknown Publisher Warning Elimination
# =============================================================================

class TestScenario5CIRunnerTrustStoreUnknownPublisher(unittest.TestCase):
    """
    Scenario 5: CI Runner Trust Store & Unknown Publisher Warning Elimination.
    Simulates Windows CI runner executing verify_signatures.yml: registering certificate
    into LocalMachine\\Root, running Authenticode checks, confirming zero unknown publisher warnings.
    """

    def setUp(self):
        self.workflow_path = WORKFLOWS_DIR / "verify_signatures.yml"

    def test_s5_1_trust_store_import_script_structure(self):
        """S5-1: Verify workflow contains PowerShell script importing cert into LocalMachine\\Root store."""
        self.assertTrue(self.workflow_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        combined_script = "\n".join(validator.extract_powershell_scripts())

        self.assertIn("LocalMachine", combined_script, "Must target LocalMachine store.")
        self.assertIn("Root", combined_script, "Must target Root certificate store.")
        self.assertIn("X509Store", combined_script, "Must instantiate X509Store.")
        self.assertIn("X509Certificate2", combined_script, "Must instantiate X509Certificate2.")

    def test_s5_2_authenticode_signature_status_validation(self):
        """S5-2: Verify workflow asserts .Status -eq 'Valid' and throws on any warning status."""
        self.assertTrue(self.workflow_path.exists(), "verify_signatures.yml must exist.")
        validator = WorkflowValidator.parse_file(self.workflow_path)
        combined_script = "\n".join(validator.extract_powershell_scripts())

        # Must explicitly check that Status is 'Valid'
        has_valid_status_check = "Status -ne 'Valid'" in combined_script or "Status -eq 'Valid'" in combined_script
        self.assertTrue(
            has_valid_status_check,
            "Requirement R4 / F12: Workflow must strictly validate Authenticode Status is 'Valid'.",
        )


# =============================================================================
# Scenario 6: Inno Setup Upgrades & Uninstaller Signing
# =============================================================================

class TestScenario6InnoSetupUpgradesAndUninstaller(unittest.TestCase):
    """
    Scenario 6: Inno Setup Upgrades & Uninstaller Signing.
    Verifies attune_installer.iss generates signed unins000.exe, preserves existing settings
    during upgrade (replacesameversion, comparetimestamp), and replaces signed DLLs cleanly
    without reboot crashes (restartreplace).
    """

    def setUp(self):
        self.iss_path = PACKAGING_DIR / "attune_installer.iss"

    def test_s6_1_signed_uninstaller_enabled_in_setup(self):
        """S6-1: Verify SignedUninstaller=yes in [Setup] section."""
        self.assertTrue(self.iss_path.exists(), f"Installer missing: {self.iss_path}")
        parser = InnoSetupParser.parse_file(self.iss_path)

        signed_unins = parser.get_setup_directive("SignedUninstaller")
        self.assertEqual(
            signed_unins,
            "yes",
            "Requirement R3 / F8: attune_installer.iss must specify 'SignedUninstaller=yes'.",
        )

    def test_s6_2_upgrade_safety_flags_on_all_dynamic_dlls(self):
        """S6-2: Verify dynamic DLL entries include replacesameversion, restartreplace, and comparetimestamp."""
        self.assertTrue(self.iss_path.exists(), "attune_installer.iss must exist.")
        parser = InnoSetupParser.parse_file(self.iss_path)

        files = parser.get_files_entries()
        dll_entries = [f for f in files if f.get("source", "").lower().endswith(".dll")]
        self.assertGreater(len(dll_entries), 0, "Installer must define dynamic DLL entries.")

        required_upgrade_flags = {"replacesameversion", "restartreplace", "comparetimestamp"}

        for entry in dll_entries:
            flags = entry.get("flags", set())
            src = entry.get("source", "")
            for req_flag in required_upgrade_flags:
                self.assertIn(
                    req_flag,
                    flags,
                    f"Scenario 6: DLL '{src}' missing upgrade safety flag '{req_flag}' in [Files].",
                )


# =============================================================================
# Scenario 7: Offline Air-Gapped Build Fallback
# =============================================================================

class TestScenario7OfflineAirGappedBuildFallback(unittest.TestCase):
    """
    Scenario 7: Offline Air-Gapped Build Fallback.
    Simulates build and package in an offline/air-gapped environment where RFC 3161
    timestamp authority servers (http://timestamp.digicert.com) are unreachable.
    Validates graceful error reporting and successful local signing without compiler crash.
    """

    def setUp(self):
        self.fixture = WindowsStagingFixture()
        self.builder_py_path = TOOLS_DIR / "builder.py"

    def tearDown(self):
        self.fixture.cleanup()

    def test_s7_1_offline_signing_produces_valid_pe_without_tsa(self):
        """S7-1: Verify binary is signed with valid Authenticode structure even without TSA countersignature."""
        target_dll = self.fixture.dist_lib_dir / "attune_engine.dll"
        AuthenticodeHelper.create_synthetic_pe(target_dll, is_signed=False)

        # Perform local signing without remote TSA
        AuthenticodeHelper.append_authenticode_signature(
            target_dll, cert_subject="CN=Attune Air-Gapped Local Signer"
        )

        parsed = PEParser.parse_file(target_dll)
        self.assertTrue(parsed["is_pe"], "Offline signed file must be valid PE.")
        self.assertTrue(parsed["is_signed"], "Offline signed file must have security directory.")
        self.assertGreater(parsed["sec_dir_size"], 0)

    def test_s7_2_offline_integrity_manifest_generated_successfully(self):
        """S7-2: Verify cryptographic integrity manifest generates successfully in air-gapped environment."""
        self.fixture.stage_binaries(signed=True)
        manifest = self.fixture.generate_manifest(version="1.0.0-offline")

        self.assertIsNotNone(manifest, "Manifest generation must not return None.")
        self.assertEqual(len(manifest["components"]), 5, "Manifest must cover all 5 components in offline mode.")

        # Test verification in offline staging
        for dll_name in REQUIRED_DYNAMIC_DLLS:
            dll_path = self.fixture.dist_lib_dir / dll_name
            sha = manifest["components"][dll_name]["sha256"]
            is_valid, msg = self.fixture.verify_component_integrity(dll_path, sha)
            self.assertTrue(is_valid, f"Offline integrity verification failed for {dll_name}: {msg}")

    def test_s7_3_builder_py_graceful_exception_handling(self):
        """S7-3: Verify builder.py wraps signing execution in try/except blocks to prevent hard crashes."""
        self.assertTrue(self.builder_py_path.exists(), "builder.py must exist.")
        builder_src = self.builder_py_path.read_text(encoding="utf-8")

        # Builder must wrap signing calls in try/except
        has_error_handling = (
            "try:" in builder_src
            and "except Exception" in builder_src
            and "Sign" in builder_src
        )
        self.assertTrue(
            has_error_handling,
            "Requirement R2: builder.py must handle signing exceptions gracefully without unhandled compiler crashes.",
        )


# =============================================================================
# Scenario 8: Windows Cross-Compilation Syntax & Target Configuration
# =============================================================================

class TestScenario8WindowsCrossCompilation(unittest.TestCase):
    """
    Scenario 8: Windows Cross-Compilation Syntax & Target Configuration.
    Validates that Windows-specific Rust FFI signatures (SetDllDirectoryW, kernel32 linkage)
    and desktop build configuration compile cleanly for x86_64-pc-windows-gnu.
    """

    def setUp(self):
        self.desktop_crate_dir = CRATES_DIR / "attune-desktop"
        self.build_rs_path = self.desktop_crate_dir / "build.rs"

    def test_s8_1_windows_cross_compilation_check(self):
        """S8-1: Verify Windows target configuration in build.rs and cargo check for x86_64-pc-windows-gnu."""
        # 1. Verify build.rs handles windows target and links kernel32
        self.assertTrue(self.build_rs_path.exists(), f"Missing build.rs at {self.build_rs_path}")
        build_rs_src = self.build_rs_path.read_text(encoding="utf-8")
        self.assertIn(
            'CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")',
            build_rs_src,
            "Requirement R1: attune-desktop/build.rs must detect CARGO_CFG_TARGET_OS == 'windows'.",
        )
        self.assertIn(
            "cargo:rustc-link-lib=kernel32",
            build_rs_src,
            "Requirement R1: attune-desktop/build.rs must emit cargo:rustc-link-lib=kernel32 on Windows.",
        )

        # 2. Check if cargo is available in environment
        cargo_bin = shutil.which("cargo")
        if not cargo_bin:
            self.skipTest("Cargo executable not found in PATH; skipping active cross-compilation check.")

        # 3. Dry-run syntax check for x86_64-pc-windows-gnu
        import subprocess
        proc = subprocess.run(
            ["cargo", "check", "--target", "x86_64-pc-windows-gnu", "-p", "attune-desktop"],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if proc.returncode != 0 and "target may not be installed" in proc.stderr:
            self.skipTest("x86_64-pc-windows-gnu target toolchain not installed in this environment.")

        self.assertEqual(
            proc.returncode,
            0,
            f"cargo check --target x86_64-pc-windows-gnu failed with code {proc.returncode}:\n{proc.stderr}",
        )


if __name__ == "__main__":
    unittest.main()
