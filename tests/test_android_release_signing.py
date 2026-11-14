#!/usr/bin/env python3
"""
Unit & Regression Tests for Android Release Signing & Keystore Generation
-------------------------------------------------------------------------
Covers the security & fail-closed guarantees around `alert-autofix-1`:
1. `tools/generate_keystore.py`: no hardcoded fallback passwords, uses
   `-storepass:env` / `-keypass:env` instead of cleartext CLI args, and
   omits `storePassword` / `keyPassword` from `key.properties`.
2. `tools/builder.py`: `validate_android_release_signing` fails closed on
   release builds when `key.properties` omits passwords or credentials cannot
   be resolved, while keeping debug builds unblocked.
3. `apps/attune_app/android/app/build.gradle.kts` and
   `.github/workflows/build_and_release.yml`: fail-closed release task guard
   and secret-free `key.properties` generation in CI.
"""

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "tools"))

import builder  # noqa: E402
import generate_keystore  # noqa: E402


class TestGenerateKeystoreSecurity(unittest.TestCase):
    def test_rejects_missing_password_without_hardcoded_fallback(self):
        with tempfile.TemporaryDirectory() as tmp:
            ks_path = Path(tmp) / "test.jks"
            env_clean = {
                k: v
                for k, v in os.environ.items()
                if k
                not in (
                    "RELEASE_KEYSTORE_PASSWORD",
                    "ATTUNE_KEYSTORE_PASSWORD",
                    "RELEASE_KEY_PASSWORD",
                    "ATTUNE_KEY_PASSWORD",
                )
            }
            with mock.patch.dict(os.environ, env_clean, clear=True), mock.patch(
                "generate_keystore.find_keytool", return_value="/usr/bin/keytool"
            ), mock.patch("sys.stdin.isatty", return_value=False):
                with self.assertRaises(ValueError) as ctx:
                    generate_keystore.generate_keystore(
                        out_path=str(ks_path),
                        create_properties=False,
                        force=True,
                    )
                self.assertIn("minimum 6 characters", str(ctx.exception))

    def test_uses_env_password_flags_and_omits_passwords_from_key_properties(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            ks_path = tmp_path / "app" / "attune_release.jks"
            props_path = tmp_path / "key.properties"

            captured = {}

            def fake_run(cmd, stdout=None, stderr=None, text=None, env=None):
                captured["cmd"] = cmd
                captured["env"] = env
                ks_path.parent.mkdir(parents=True, exist_ok=True)
                ks_path.write_bytes(b"fake-pkcs12-keystore")
                res = mock.MagicMock()
                res.returncode = 0
                res.stderr = ""
                return res

            with mock.patch("generate_keystore.find_keytool", return_value="/usr/bin/keytool"), mock.patch(
                "generate_keystore.DEFAULT_KEY_PROPERTIES", props_path
            ), mock.patch("generate_keystore.subprocess.run", side_effect=fake_run):
                generate_keystore.generate_keystore(
                    out_path=str(ks_path),
                    alias="attune",
                    storepass="S3cureStorePass!",
                    keypass="S3cureKeyPass!",
                    create_properties=True,
                    force=True,
                    log_func=lambda _m: None,
                )

            cmd = captured["cmd"]
            self.assertIn("-storepass:env", cmd)
            self.assertIn("-keypass:env", cmd)
            self.assertNotIn("-storepass", cmd)
            self.assertNotIn("-keypass", cmd)
            self.assertNotIn("S3cureStorePass!", cmd)
            self.assertNotIn("S3cureKeyPass!", cmd)
            self.assertEqual(captured["env"]["ATTUNE_KEYTOOL_STOREPASS"], "S3cureStorePass!")
            self.assertEqual(captured["env"]["ATTUNE_KEYTOOL_KEYPASS"], "S3cureKeyPass!")

            props_text = props_path.read_text(encoding="utf-8")
            self.assertIn("storeFile=", props_text)
            self.assertIn("keyAlias=attune", props_text)
            self.assertNotIn("storePassword=", props_text)
            self.assertNotIn("keyPassword=", props_text)


class TestBuilderAndroidReleaseSigning(unittest.TestCase):
    def _clean_signing_env(self):
        return {
            k: v
            for k, v in os.environ.items()
            if k
            not in (
                "RELEASE_KEYSTORE_FILE",
                "ATTUNE_KEYSTORE_FILE",
                "RELEASE_KEYSTORE_PASSWORD",
                "ATTUNE_KEYSTORE_PASSWORD",
                "RELEASE_KEY_ALIAS",
                "ATTUNE_KEY_ALIAS",
                "RELEASE_KEY_PASSWORD",
                "ATTUNE_KEY_PASSWORD",
                "ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE",
            )
        }

    def test_fails_when_key_properties_has_no_passwords_even_with_debug_opt_in(self):
        with tempfile.TemporaryDirectory() as tmp:
            flutter_dir = Path(tmp)
            android_dir = flutter_dir / "android"
            app_dir = android_dir / "app"
            app_dir.mkdir(parents=True)
            (app_dir / "attune_release.jks").write_bytes(b"ks")
            (android_dir / "key.properties").write_text(
                "storeFile=app/attune_release.jks\nkeyAlias=attune\n",
                encoding="utf-8",
            )

            env = self._clean_signing_env()
            env["ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE"] = "true"
            with mock.patch.dict(os.environ, env, clear=True):
                with self.assertRaises(RuntimeError) as ctx:
                    builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)
                self.assertIn("Refusing to silently fall back to debug signing", str(ctx.exception))
                self.assertIn("storePassword", str(ctx.exception))

    def test_succeeds_with_passwordless_key_properties_and_env_secrets(self):
        with tempfile.TemporaryDirectory() as tmp:
            flutter_dir = Path(tmp)
            android_dir = flutter_dir / "android"
            app_dir = android_dir / "app"
            app_dir.mkdir(parents=True)
            (app_dir / "attune_release.jks").write_bytes(b"ks")
            (android_dir / "key.properties").write_text(
                "storeFile=app/attune_release.jks\nkeyAlias=attune\n",
                encoding="utf-8",
            )

            env = self._clean_signing_env()
            env["RELEASE_KEYSTORE_PASSWORD"] = "EnvStorePass99"
            env["RELEASE_KEY_PASSWORD"] = "EnvKeyPass99"
            with mock.patch.dict(os.environ, env, clear=True):
                res = builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)
                self.assertEqual(res["mode"], "release")
                self.assertEqual(res["alias"], "attune")

    def test_fails_without_any_keystore_unless_explicitly_opted_in(self):
        with tempfile.TemporaryDirectory() as tmp:
            flutter_dir = Path(tmp)
            (flutter_dir / "android").mkdir(parents=True)

            env = self._clean_signing_env()
            with mock.patch.dict(os.environ, env, clear=True):
                with self.assertRaises(RuntimeError):
                    builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)

            env["ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE"] = "1"
            with mock.patch.dict(os.environ, env, clear=True):
                res = builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)
                self.assertEqual(res["mode"], "debug_fallback")

            env["ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE"] = "  Yes  "
            with mock.patch.dict(os.environ, env, clear=True):
                res = builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)
                self.assertEqual(res["mode"], "debug_fallback")

    def test_java_properties_syntax_compatibility(self):
        with tempfile.TemporaryDirectory() as tmp:
            flutter_dir = Path(tmp)
            android_dir = flutter_dir / "android"
            app_dir = android_dir / "app"
            app_dir.mkdir(parents=True)
            (app_dir / "attune_release.jks").write_bytes(b"ks")

            props_file = android_dir / "key.properties"
            props_file.write_text(
                "# comment ending with \\\n"
                "! another comment\n"
                "storeFile app/\\\n"
                "    attune_release.jks\n"
                "keyAlias attune\n"
                "escaped\\=key: val\\:1\n"
                "trailingBackslash=C\\\\\n"
                "unicodeVal = \\u0041BC\n",
                encoding="utf-8",
            )

            parsed = builder._parse_key_properties(props_file)
            self.assertEqual(parsed["storeFile"], "app/attune_release.jks")
            self.assertEqual(parsed["keyAlias"], "attune")
            self.assertEqual(parsed["escaped=key"], "val:1")
            self.assertEqual(parsed["trailingBackslash"], "C\\")
            self.assertEqual(parsed["unicodeVal"], "ABC")

            env = self._clean_signing_env()
            env["RELEASE_KEYSTORE_PASSWORD"] = "EnvStorePass99"
            env["RELEASE_KEY_PASSWORD"] = "EnvKeyPass99"
            with mock.patch.dict(os.environ, env, clear=True):
                res = builder.validate_android_release_signing(flutter_dir, log_func=lambda _m: None)
                self.assertEqual(res["mode"], "release")
                self.assertEqual(res["alias"], "attune")

    def test_debug_compile_flutter_does_not_require_release_credentials(self):
        with tempfile.TemporaryDirectory() as tmp:
            flutter_dir = Path(tmp)
            android_dir = flutter_dir / "android"
            android_dir.mkdir(parents=True)
            (android_dir / "key.properties").write_text(
                "storeFile=app/attune_release.jks\nkeyAlias=attune\n",
                encoding="utf-8",
            )

            env = self._clean_signing_env()
            mock_proc = mock.MagicMock()
            mock_proc.stdout = iter(["Built build/app/outputs/flutter-apk/app-debug.apk\n"])
            mock_proc.returncode = 0
            mock_proc.wait.return_value = 0

            with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(
                builder, "REPO_ROOT", flutter_dir.parent.parent
            ), mock.patch("builder.get_pubspec_version", return_value=("0.1.1", "10")), mock.patch(
                "builder.sync_pubspec_version"
            ), mock.patch("builder.subprocess.Popen", return_value=mock_proc):
                # Calling compile_flutter with mode="debug" must not fail on missing release passwords
                builder.compile_flutter(target_os="android", mode="debug", log_callback=lambda _m: None)


class TestGradleAndWorkflowSigningContracts(unittest.TestCase):
    def test_build_gradle_fails_closed_for_release_tasks(self):
        gradle_text = (REPO_ROOT / "apps" / "attune_app" / "android" / "app" / "build.gradle.kts").read_text(
            encoding="utf-8"
        )
        self.assertIn("throw GradleException(", gradle_text)
        self.assertIn("hasCompleteReleaseSigning", gradle_text)
        self.assertIn("ATTUNE_ALLOW_DEBUG_SIGNING_FOR_RELEASE", gradle_text)
        self.assertIn('"1", "true", "yes"', gradle_text)
        self.assertIn('signingConfig = signingConfigs.getByName("debug")', gradle_text)

    def test_ci_workflow_valid_and_has_no_hardcoded_keystore_password(self):
        wf_path = REPO_ROOT / ".github" / "workflows" / "build_and_release.yml"
        wf_text = wf_path.read_text(encoding="utf-8")
        self.assertNotIn("attunerelease", wf_text)
        parsed = yaml.safe_load(wf_text)
        self.assertIsInstance(parsed, dict)
        build_steps = parsed["jobs"]["build"]["steps"]
        android_build_step = next(s for s in build_steps if s.get("name") == "Build, Test & Package (Android)")
        self.assertIn("RELEASE_KEYSTORE_PASSWORD", android_build_step.get("env", {}))
        self.assertIn("RELEASE_KEY_PASSWORD", android_build_step.get("env", {}))


if __name__ == "__main__":
    unittest.main()
