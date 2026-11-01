#!/usr/bin/env python3
"""
Attune Android Release Keystore Generator
------------------------------------------
Generates a persistent PKCS12 release keystore for Android APK signing using keytool.
Outputs the keystore file and generates companion key.properties for Gradle builds.

Distributed under the MIT License.
"""

import argparse
import getpass
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_KEYSTORE_PATH = REPO_ROOT / "apps" / "attune_app" / "android" / "app" / "attune_release.jks"
DEFAULT_KEY_PROPERTIES = REPO_ROOT / "apps" / "attune_app" / "android" / "key.properties"

DEFAULT_DNAME = "CN=Attune, OU=MizProject Organization, O=Attune Audio, L=Zamboanga Peninsula, ST=Zamboanga Peninsula, C=PH"


def find_keytool():
    """Locates the Java keytool binary from PATH or JAVA_HOME."""
    tool = shutil.which("keytool")
    if tool:
        return tool

    java_home = os.environ.get("JAVA_HOME")
    if java_home:
        cand = Path(java_home) / "bin" / ("keytool.exe" if sys.platform == "win32" else "keytool")
        if cand.exists():
            return str(cand)

    # Standard Linux / macOS candidates
    linux_cands = [
        Path("/usr/lib/jvm/default/bin/keytool"),
        Path("/usr/lib/jvm/java-21-openjdk/bin/keytool"),
        Path("/usr/lib/jvm/java-17-openjdk/bin/keytool"),
    ]
    for c in linux_cands:
        if c.exists():
            return str(c)

    return None


def generate_keystore(
    out_path=None,
    alias=None,
    storepass=None,
    keypass=None,
    dname=None,
    validity_days=10000,
    create_properties=True,
    force=False,
    log_func=print,
):
    """Generates a PKCS12 keystore using keytool and writes key.properties."""
    keytool_bin = find_keytool()
    if not keytool_bin:
        raise RuntimeError("keytool binary not found. Please install a JDK and ensure keytool or JAVA_HOME is in PATH.")

    target_keystore = Path(out_path or os.environ.get("ATTUNE_KEYSTORE_FILE") or DEFAULT_KEYSTORE_PATH).resolve()
    target_alias = alias or os.environ.get("ATTUNE_KEY_ALIAS") or "attune"

    if target_keystore.exists() and not force:
        log_func(f"[Keystore] Keystore already exists at: {target_keystore}")
        log_func("  Use --force to overwrite if you wish to regenerate.")
        return target_keystore

    # Resolve passwords
    sp = storepass or os.environ.get("ATTUNE_KEYSTORE_PASSWORD")
    if not sp:
        if sys.stdin.isatty():
            sp = getpass.getpass("Enter Keystore Password (min 6 chars): ").strip()
        if not sp:
            sp = "attunerelease"
            log_func("[Keystore] Using default release password for local development.")

    kp = keypass or os.environ.get("ATTUNE_KEY_PASSWORD") or sp

    target_dname = dname or os.environ.get("ATTUNE_KEY_DNAME") or DEFAULT_DNAME

    target_keystore.parent.mkdir(parents=True, exist_ok=True)
    if target_keystore.exists() and force:
        target_keystore.unlink()

    cmd = [
        keytool_bin,
        "-genkeypair",
        "-v",
        "-keystore", str(target_keystore),
        "-storetype", "PKCS12",
        "-alias", target_alias,
        "-keyalg", "RSA",
        "-keysize", "2048",
        "-validity", str(validity_days),
        "-dname", target_dname,
        "-storepass", sp,
        "-keypass", kp,
    ]

    log_func(f"[Keystore] Generating {validity_days}-day PKCS12 keystore at: {target_keystore} (alias: {target_alias})")
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"keytool failed with exit code {proc.returncode}:\n{proc.stderr}")

    log_func(f"[Keystore] Successfully created keystore: {target_keystore}")

    if create_properties:
        rel_store_file = os.path.relpath(target_keystore, DEFAULT_KEY_PROPERTIES.parent)
        prop_content = (
            f"storeFile={rel_store_file}\n"
            f"storePassword={sp}\n"
            f"keyAlias={target_alias}\n"
            f"keyPassword={kp}\n"
        )
        DEFAULT_KEY_PROPERTIES.write_text(prop_content, encoding="utf-8")
        log_func(f"[Keystore] Generated companion config: {DEFAULT_KEY_PROPERTIES}")
        log_func("  Note: Both the keystore and key.properties are strictly ignored by .gitignore to preserve privacy.")

    return target_keystore


def main():
    parser = argparse.ArgumentParser(description="Generate PKCS12 release keystore for Attune Android app.")
    parser.add_argument("--out", "-o", help="Target output keystore path (default: apps/attune_app/android/app/attune_release.jks)")
    parser.add_argument("--alias", "-a", help="Key alias name (default: attune)")
    parser.add_argument("--storepass", "-p", help="Keystore password (or env ATTUNE_KEYSTORE_PASSWORD)")
    parser.add_argument("--keypass", help="Key password (defaults to storepass)")
    parser.add_argument("--dname", "-d", help="Distinguished Name string (CN=..., OU=..., etc.)")
    parser.add_argument("--validity", type=int, default=10000, help="Validity in days (default: 10000)")
    parser.add_argument("--no-key-properties", action="store_true", help="Skip creating key.properties file")
    parser.add_argument("--force", "-f", action="store_true", help="Force overwrite existing keystore")

    args = parser.parse_args()

    try:
        generate_keystore(
            out_path=args.out,
            alias=args.alias,
            storepass=args.storepass,
            keypass=args.keypass,
            dname=args.dname,
            validity_days=args.validity,
            create_properties=not args.no_key_properties,
            force=args.force,
        )
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
