#!/usr/bin/env python3
"""
Attune Android Release Keystore Generator (GUI & CLI)
------------------------------------------------------
Generates a persistent PKCS12 release keystore for Android APK signing using keytool.
Features both a modern Tkinter GUI and a headless CLI.
Allows copying base64-encoded keystores directly for GitHub Actions CI secrets
(RELEASE_KEYSTORE_BASE64).

Distributed under the MIT License.
"""

import argparse
import base64
import getpass
import os
import platform
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


def get_keystore_base64(keystore_path):
    """Reads a keystore file and returns its base64-encoded string."""
    p = Path(keystore_path)
    if not p.exists():
        raise FileNotFoundError(f"Keystore file not found at: {p}")
    data = p.read_bytes()
    return base64.b64encode(data).decode("utf-8")


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
    """Generates a PKCS12 keystore using keytool and optionally writes key.properties."""
    keytool_bin = find_keytool()
    if not keytool_bin:
        raise RuntimeError("keytool binary not found. Please install a JDK and ensure keytool or JAVA_HOME is in PATH.")

    target_keystore = Path(out_path or os.environ.get("RELEASE_KEYSTORE_FILE") or os.environ.get("ATTUNE_KEYSTORE_FILE") or DEFAULT_KEYSTORE_PATH).resolve()
    target_alias = alias or os.environ.get("RELEASE_KEY_ALIAS") or os.environ.get("ATTUNE_KEY_ALIAS") or "attune"

    if target_keystore.exists() and not force:
        log_func(f"[Keystore] Keystore already exists at: {target_keystore}")
        log_func("  Use --force or check 'Overwrite' in GUI if you wish to regenerate.")
        b64 = get_keystore_base64(target_keystore)
        return target_keystore, b64

    # Resolve passwords
    sp = storepass or os.environ.get("RELEASE_KEYSTORE_PASSWORD") or os.environ.get("ATTUNE_KEYSTORE_PASSWORD")
    if not sp:
        if sys.stdin.isatty():
            sp = getpass.getpass("Enter Keystore Password (min 6 chars): ").strip()
        if not sp:
            sp = "attunerelease"
            log_func("[Keystore] Using default release password for local development.")

    kp = keypass or os.environ.get("RELEASE_KEY_PASSWORD") or os.environ.get("ATTUNE_KEY_PASSWORD") or sp

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
            f"keyAlias={target_alias}\n"
        )
        DEFAULT_KEY_PROPERTIES.write_text(prop_content, encoding="utf-8")
        log_func(f"[Keystore] Generated companion config: {DEFAULT_KEY_PROPERTIES}")
        log_func("  Note: Passwords are not written to key.properties; set RELEASE_KEYSTORE_PASSWORD/ATTUNE_KEYSTORE_PASSWORD and RELEASE_KEY_PASSWORD/ATTUNE_KEY_PASSWORD in your environment.")

    b64_val = get_keystore_base64(target_keystore)
    return target_keystore, b64_val


# ==============================================================================
# Interactive Tkinter GUI Mode
# ==============================================================================

def launch_gui():
    """Launches the interactive Tkinter desktop GUI for keystore generation."""
    try:
        import tkinter as tk
        from tkinter import ttk, messagebox, filedialog, scrolledtext
    except ImportError:
        print("ERROR: Tkinter is not installed on this system.")
        print("Run with --no-gui for command-line mode.")
        sys.exit(1)

    root = tk.Tk()
    root.title("Attune - Android Release Keystore Generator")
    root.geometry("820x760")
    root.minsize(740, 620)

    # Theme colors
    bg_color = "#161922"
    fg_color = "#E0E3EC"
    accent_color = "#68D391"
    card_color = "#1E222D"
    input_bg = "#252A38"
    btn_bg = "#2D3748"

    root.configure(bg=bg_color)

    style = ttk.Style()
    try:
        style.theme_use("clam")
    except Exception:
        pass
    style.configure(".", background=bg_color, foreground=fg_color)
    style.configure("TLabel", background=bg_color, foreground=fg_color, font=("Segoe UI", 10))
    style.configure("Header.TLabel", font=("Segoe UI", 16, "bold"), foreground=accent_color)
    style.configure("SubHeader.TLabel", font=("Segoe UI", 10), foreground="#A0AEC0")
    style.configure("Card.TFrame", background=card_color)
    style.configure("CardLabel.TLabel", background=card_color, foreground=fg_color, font=("Segoe UI", 10))
    style.configure("CardHeader.TLabel", background=card_color, foreground=accent_color, font=("Segoe UI", 11, "bold"))
    style.configure("TCheckbutton", background=card_color, foreground=fg_color, font=("Segoe UI", 10))

    main_frame = ttk.Frame(root, padding=16)
    main_frame.pack(fill=tk.BOTH, expand=True)

    # Header
    header_frame = ttk.Frame(main_frame)
    header_frame.pack(fill=tk.X, pady=(0, 12))
    ttk.Label(header_frame, text="🔑 Android Release Keystore Studio", style="Header.TLabel").pack(anchor=tk.W)
    ttk.Label(
        header_frame,
        text="Generate PKCS12 release keystores and export RELEASE_KEYSTORE_BASE64 for GitHub Actions CI",
        style="SubHeader.TLabel",
    ).pack(anchor=tk.W, pady=(2, 0))

    # Form Card
    form_card = ttk.Frame(main_frame, style="Card.TFrame", padding=14)
    form_card.pack(fill=tk.X, pady=(0, 10))

    ttk.Label(form_card, text="Keystore Configuration", style="CardHeader.TLabel").grid(row=0, column=0, columnspan=3, sticky=tk.W, pady=(0, 8))

    # File Path
    ttk.Label(form_card, text="Destination Path:", style="CardLabel.TLabel").grid(row=1, column=0, sticky=tk.W, pady=4)
    path_var = tk.StringVar(value=str(DEFAULT_KEYSTORE_PATH))
    path_entry = tk.Entry(form_card, textvariable=path_var, width=54, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT)
    path_entry.grid(row=1, column=1, sticky=tk.EW, padx=(6, 6), pady=4)

    def browse_path():
        initial_dir = str(DEFAULT_KEYSTORE_PATH.parent)
        f = filedialog.asksaveasfilename(
            initialdir=initial_dir,
            initialfile=DEFAULT_KEYSTORE_PATH.name,
            defaultextension=".jks",
            filetypes=[("Java Keystore", "*.jks"), ("All Files", "*.*")],
        )
        if f:
            path_var.set(f)

    browse_btn = tk.Button(form_card, text="Browse...", command=browse_path, bg=btn_bg, fg=fg_color, relief=tk.FLAT, padx=8)
    browse_btn.grid(row=1, column=2, padx=(2, 0), pady=4)

    # Key Alias
    ttk.Label(form_card, text="Key Alias:", style="CardLabel.TLabel").grid(row=2, column=0, sticky=tk.W, pady=4)
    alias_var = tk.StringVar(value="attune")
    alias_entry = tk.Entry(form_card, textvariable=alias_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT)
    alias_entry.grid(row=2, column=1, sticky=tk.W, padx=6, pady=4)

    # Passwords
    ttk.Label(form_card, text="Store Password:", style="CardLabel.TLabel").grid(row=3, column=0, sticky=tk.W, pady=4)
    storepass_var = tk.StringVar(value="attunerelease")
    storepass_entry = tk.Entry(form_card, textvariable=storepass_var, show="*", width=24, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT)
    storepass_entry.grid(row=3, column=1, sticky=tk.W, padx=6, pady=4)

    ttk.Label(form_card, text="Key Password:", style="CardLabel.TLabel").grid(row=4, column=0, sticky=tk.W, pady=4)
    keypass_var = tk.StringVar(value="attunerelease")
    keypass_entry = tk.Entry(form_card, textvariable=keypass_var, show="*", width=24, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT)
    keypass_entry.grid(row=4, column=1, sticky=tk.W, padx=6, pady=4)

    same_pass_var = tk.BooleanVar(value=True)

    def on_same_pass_toggle():
        if same_pass_var.get():
            keypass_var.set(storepass_var.get())
            keypass_entry.config(state="disabled")
        else:
            keypass_entry.config(state="normal")

    same_pass_check = ttk.Checkbutton(form_card, text="Same password for key and store", variable=same_pass_var, command=on_same_pass_toggle)
    same_pass_check.grid(row=4, column=2, sticky=tk.W, padx=4)
    on_same_pass_toggle()

    # DNAME Card
    dname_card = ttk.Frame(main_frame, style="Card.TFrame", padding=14)
    dname_card.pack(fill=tk.X, pady=(0, 10))

    ttk.Label(dname_card, text="Distinguished Name (Certificate Ownership)", style="CardHeader.TLabel").grid(row=0, column=0, columnspan=4, sticky=tk.W, pady=(0, 8))

    cn_var = tk.StringVar(value="Attune")
    ou_var = tk.StringVar(value="MizProject Organization")
    o_var = tk.StringVar(value="Attune Audio")
    l_var = tk.StringVar(value="Zamboanga Peninsula")
    st_var = tk.StringVar(value="Zamboanga Peninsula")
    c_var = tk.StringVar(value="PH")
    validity_var = tk.StringVar(value="10000")

    ttk.Label(dname_card, text="Common Name (CN):", style="CardLabel.TLabel").grid(row=1, column=0, sticky=tk.W, pady=3)
    tk.Entry(dname_card, textvariable=cn_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=1, column=1, sticky=tk.EW, padx=4, pady=3)

    ttk.Label(dname_card, text="Org Unit (OU):", style="CardLabel.TLabel").grid(row=1, column=2, sticky=tk.W, padx=(12, 4), pady=3)
    tk.Entry(dname_card, textvariable=ou_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=1, column=3, sticky=tk.EW, padx=4, pady=3)

    ttk.Label(dname_card, text="Organization (O):", style="CardLabel.TLabel").grid(row=2, column=0, sticky=tk.W, pady=3)
    tk.Entry(dname_card, textvariable=o_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=2, column=1, sticky=tk.EW, padx=4, pady=3)

    ttk.Label(dname_card, text="City / Locality (L):", style="CardLabel.TLabel").grid(row=2, column=2, sticky=tk.W, padx=(12, 4), pady=3)
    tk.Entry(dname_card, textvariable=l_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=2, column=3, sticky=tk.EW, padx=4, pady=3)

    ttk.Label(dname_card, text="State / Province (ST):", style="CardLabel.TLabel").grid(row=3, column=0, sticky=tk.W, pady=3)
    tk.Entry(dname_card, textvariable=st_var, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=3, column=1, sticky=tk.EW, padx=4, pady=3)

    ttk.Label(dname_card, text="Country Code (C):", style="CardLabel.TLabel").grid(row=3, column=2, sticky=tk.W, padx=(12, 4), pady=3)
    tk.Entry(dname_card, textvariable=c_var, width=6, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=3, column=3, sticky=tk.W, padx=4, pady=3)

    ttk.Label(dname_card, text="Validity (Days):", style="CardLabel.TLabel").grid(row=4, column=0, sticky=tk.W, pady=3)
    tk.Entry(dname_card, textvariable=validity_var, width=10, bg=input_bg, fg=fg_color, insertbackground=fg_color, relief=tk.FLAT).grid(row=4, column=1, sticky=tk.W, padx=4, pady=3)

    # Options
    opts_frame = ttk.Frame(dname_card, style="Card.TFrame")
    opts_frame.grid(row=5, column=0, columnspan=4, sticky=tk.W, pady=(8, 0))

    create_props_var = tk.BooleanVar(value=True)
    ttk.Checkbutton(opts_frame, text="Generate companion key.properties (git-ignored)", variable=create_props_var).pack(side=tk.LEFT, padx=(0, 14))

    force_var = tk.BooleanVar(value=False)
    ttk.Checkbutton(opts_frame, text="Overwrite if keystore exists", variable=force_var).pack(side=tk.LEFT)

    # Button Bar
    btn_bar = ttk.Frame(main_frame)
    btn_bar.pack(fill=tk.X, pady=(0, 10))

    last_base64 = [""]

    def log_gui(msg):
        log_text.insert(tk.END, msg + "\n")
        log_text.see(tk.END)
        root.update_idletasks()

    def do_generate():
        log_text.delete("1.0", tk.END)
        dname_str = f"CN={cn_var.get().strip()}, OU={ou_var.get().strip()}, O={o_var.get().strip()}, L={l_var.get().strip()}, ST={st_var.get().strip()}, C={c_var.get().strip()}"
        sp = storepass_var.get().strip()
        kp = sp if same_pass_var.get() else keypass_var.get().strip()

        try:
            val_days = int(validity_var.get().strip())
        except ValueError:
            val_days = 10000

        try:
            target_ks, b64_str = generate_keystore(
                out_path=path_var.get().strip(),
                alias=alias_var.get().strip(),
                storepass=sp,
                keypass=kp,
                dname=dname_str,
                validity_days=val_days,
                create_properties=create_props_var.get(),
                force=force_var.get(),
                log_func=log_gui,
            )
            last_base64[0] = b64_str
            log_gui("\n" + "=" * 60)
            log_gui("🎉 Keystore generation complete!")
            log_gui("To configure GitHub Actions CI for release builds:")
            log_gui("  1. Go to GitHub Repo -> Settings -> Secrets and variables -> Actions")
            log_gui("  2. Click 'Copy Base64 for RELEASE_KEYSTORE_BASE64' below")
            log_gui("  3. Create repository secret: RELEASE_KEYSTORE_BASE64")
            log_gui(f"  4. Create repository secret: RELEASE_KEYSTORE_PASSWORD = {sp}")
            log_gui(f"  5. Create repository secret: RELEASE_KEY_ALIAS = {alias_var.get().strip()}")
            log_gui(f"  6. Create repository secret: RELEASE_KEY_PASSWORD = {kp}")
            log_gui("=" * 60)
            messagebox.showinfo("Success", f"Keystore successfully generated at:\n{target_ks}\n\nReady for export to GitHub Secrets!")
        except Exception as e:
            log_gui(f"\n[ERROR] {e}")
            messagebox.showerror("Error", str(e))

    def do_copy_base64():
        b64 = last_base64[0]
        if not b64:
            p = Path(path_var.get().strip())
            if p.exists():
                try:
                    b64 = get_keystore_base64(p)
                    last_base64[0] = b64
                except Exception as e:
                    messagebox.showerror("Error", f"Failed to read keystore: {e}")
                    return
            else:
                messagebox.showwarning("Warning", "Generate the keystore first or select an existing keystore file.")
                return

        root.clipboard_clear()
        root.clipboard_append(b64)
        log_gui("[Clipboard] Successfully copied RELEASE_KEYSTORE_BASE64 to clipboard!")
        messagebox.showinfo("Copied", "RELEASE_KEYSTORE_BASE64 has been copied to your clipboard.\nPaste it directly into GitHub Repository Secrets!")

    def do_save_base64():
        b64 = last_base64[0]
        if not b64:
            p = Path(path_var.get().strip())
            if p.exists():
                b64 = get_keystore_base64(p)
                last_base64[0] = b64
            else:
                messagebox.showwarning("Warning", "Generate the keystore first or select an existing keystore file.")
                return

        f = filedialog.asksaveasfilename(
            defaultextension=".txt",
            filetypes=[("Text File", "*.txt"), ("Base64 File", "*.b64"), ("All Files", "*.*")],
            initialfile="RELEASE_KEYSTORE_BASE64.txt",
        )
        if f:
            Path(f).write_text(b64, encoding="utf-8")
            log_gui(f"[Export] Saved base64 keystore to: {f}")
            messagebox.showinfo("Saved", f"Base64 file written to:\n{f}")

    gen_btn = tk.Button(btn_bar, text="🔑 Generate Keystore", command=do_generate, bg=accent_color, fg="#1A202C", font=("Segoe UI", 10, "bold"), relief=tk.FLAT, padx=14, pady=6)
    gen_btn.pack(side=tk.LEFT, padx=(0, 8))

    copy_btn = tk.Button(btn_bar, text="📋 Copy RELEASE_KEYSTORE_BASE64", command=do_copy_base64, bg=btn_bg, fg=accent_color, font=("Segoe UI", 10), relief=tk.FLAT, padx=12, pady=6)
    copy_btn.pack(side=tk.LEFT, padx=(0, 8))

    save_btn = tk.Button(btn_bar, text="💾 Save Base64...", command=do_save_base64, bg=btn_bg, fg=fg_color, font=("Segoe UI", 10), relief=tk.FLAT, padx=10, pady=6)
    save_btn.pack(side=tk.LEFT)

    # Log text
    log_frame = ttk.Frame(main_frame)
    log_frame.pack(fill=tk.BOTH, expand=True)

    log_text = scrolledtext.ScrolledText(log_frame, bg="#0D1117", fg="#C9D1D9", insertbackground="#C9D1D9", font=("Consolas", 9), relief=tk.FLAT)
    log_text.pack(fill=tk.BOTH, expand=True)

    log_gui("Attune Keystore Studio ready. Click 'Generate Keystore' to begin.")
    if Path(path_var.get()).exists():
        log_gui(f"Found existing keystore at: {path_var.get()}")
        log_gui("You can click 'Copy RELEASE_KEYSTORE_BASE64' directly to export it to GitHub Secrets.")

    root.mainloop()


# ==============================================================================
# CLI Entrypoint
# ==============================================================================

def main():
    parser = argparse.ArgumentParser(description="Generate PKCS12 release keystore for Attune Android app (GUI & CLI).")
    parser.add_argument("--gui", action="store_true", help="Launch interactive Tkinter desktop GUI")
    parser.add_argument("--no-gui", action="store_true", help="Force command-line execution")
    parser.add_argument("--out", "-o", help="Target output keystore path (default: apps/attune_app/android/app/attune_release.jks)")
    parser.add_argument("--alias", "-a", help="Key alias name (default: attune)")
    parser.add_argument("--storepass", "-p", help="Keystore password (or env ATTUNE_KEYSTORE_PASSWORD)")
    parser.add_argument("--keypass", help="Key password (defaults to storepass)")
    parser.add_argument("--dname", "-d", help="Distinguished Name string (CN=..., OU=..., etc.)")
    parser.add_argument("--validity", type=int, default=10000, help="Validity in days (default: 10000)")
    parser.add_argument("--no-key-properties", action="store_true", help="Skip creating key.properties file")
    parser.add_argument("--force", "-f", action="store_true", help="Force overwrite existing keystore")
    parser.add_argument("--base64", action="store_true", help="Print base64 representation of keystore for GitHub Secrets")

    args = parser.parse_args()

    # Determine whether to launch GUI:
    # If explicitly requested with --gui, or if run with 0 CLI arguments and DISPLAY / Windows desktop is available and not --no-gui
    has_display = bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY") or platform.system() == "Windows")
    if args.gui or (not args.no_gui and len(sys.argv) == 1 and has_display):
        launch_gui()
        return

    try:
        ks_path, b64_str = generate_keystore(
            out_path=args.out,
            alias=args.alias,
            storepass=args.storepass,
            keypass=args.keypass,
            dname=args.dname,
            validity_days=args.validity,
            create_properties=not args.no_key_properties,
            force=args.force,
        )
        if args.base64:
            print("\n--- RELEASE_KEYSTORE_BASE64 (Copy below into GitHub Repository Secrets) ---")
            print(b64_str)
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
