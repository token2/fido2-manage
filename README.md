# fido2-manage

**fido2-manage** is a command-line tool for managing FIDO2 security keys —
passkeys, PINs, fingerprints, PIN policy and factory reset. This project is the
ground-up rewrite of the original
[fido2-manage](https://github.com/token2/fido2-manage): a native, cross-platform
binary (no Python/pexpect dependency), with the classic flag set preserved.

Alongside the rewritten CLI, the project now also ships **PIV command-line tools**
and a **graphical front-end** (Token2 Key Manager) that adds PIV and OTP
management on top of fido2-manage. The GUI is secondary — fido2-manage is the
main deliverable.

> The previous C++/PowerShell implementation is preserved on the
> [`legacy`](https://github.com/token2/fido2-manage/tree/legacy) branch.

## fido2-manage — usage

    fido2-manage -list
    fido2-manage -info               -device N
    fido2-manage -storage            -device N
    fido2-manage -residentKeys       -device N [-domain DOMAIN]
    fido2-manage -delete             -device N -credential <base64>
    fido2-manage -setPIN             -device N [-pin PIN]
    fido2-manage -changePIN          -device N [-pin PIN -newpin NEWPIN]
    fido2-manage -reset              -device N
    fido2-manage -forcePINchange     -device N [-pin PIN]
    fido2-manage -setMinimumPIN M    -device N [-pin PIN]
    fido2-manage -uvs | -uvd         -device N [-pin PIN]
    fido2-manage -fingerprint        -device N [-pin PIN] [-fingerprintname NAME]
    fido2-manage -fingerprintlist    -device N [-pin PIN]
    fido2-manage -deletefingerprint ID -device N [-pin PIN]
    fido2-manage -renamefingerprint ID -fingerprintname NAME -device N [-pin PIN]

- `-device N` selects the Nth key from `-list` (1-based). Defaults to 1.
- If `-pin` is omitted, the tool prompts for it and masks the input.
- `-reset` only works shortly after the key is plugged in (CTAP power-up window):
  unplug and replug the key, run the command, and touch it when it blinks.

The flag set is **drop-in compatible** with the classic `fido2-manage.exe`, so
existing scripts keep working.

### Administrator rights (Windows)

Token2 keys expose their FIDO applet over a **smart-card (CCID/PC-SC)** interface,
so fido2-manage works **without administrator rights** on Windows for those keys
(USB and NFC). Raw-HID FIDO keys (e.g. a YubiKey over USB) need an **elevated**
terminal. On macOS and Linux no elevation is required.

## Also included

The same package bundles:

- **`token2-piv-tool`** — PIV smart-card management (certificates, keys, PIN/PUK,
  management key), plus `libt2cs11` for PKCS#11.
- **`token2-key-manager`** — the graphical app (Windows / macOS / Linux). A GUI
  front-end for fido2-manage, with added **PIV** certificate management and
  **OTP / OATH-TOTP/HOTP** account management for Token2 PIN+ and FIDO2 keys and
  other CTAP2 / PIV devices.
- `token2-fido2-token` / `-cred` / `-assert` — the upstream libfido2 tools.

## Download / install

Pre-built packages published by the maintainers on the Releases page for each
platform:

- **Windows** — portable folder `token2-key-manager\` containing `fido2-manage.exe`,
  the other CLI tools, the GUI, and all DLLs.
- **Linux** — a `.deb` (recommended; pulls in `pcscd` automatically, no FUSE
  needed), an **AppImage**, and a portable folder. See *Linux notes* below.
- **macOS** — a signed/notarized `.dmg`, and a portable folder.

In the AppImage, the CLI tools are reachable with
`./Token2-Key-Manager-x86_64.AppImage --tool fido2-manage -list` (or by symlinking
the AppImage to `fido2-manage`).

### Linux notes

FIDO2 and PIV use the PC/SC smart-card service. The `.deb` declares and enables it
for you. For the AppImage or portable build, install the prerequisites manually:

    sudo apt install pcscd libpcsclite1 libccid
    sudo systemctl enable --now pcscd

The **AppImage** additionally needs **`libfuse2`** on Ubuntu 24.04+:

    sudo apt install libfuse2

…or run it without FUSE: `./Token2-Key-Manager-x86_64.AppImage --appimage-extract-and-run`.
The `.deb` avoids both requirements.

## Building from source

### Windows (one command)

Install once: Visual Studio 2022 with "Desktop development with C++", Git, Rust
(rustup, MSVC). Then from this folder in PowerShell:

    Set-ExecutionPolicy -Scope Process Bypass
    .\build-all.ps1

(or `build-all.cmd` from cmd.exe). vcpkg and the Tauri CLI are fetched
automatically. Output lands in `dist\token2-key-manager\` — `fido2-manage.exe`,
the other CLI tools, the GUI, and all DLLs together.

Options: `-Dev` runs the GUI in dev mode, `-FastDev` for a quick incremental GUI
build, `-ToolOnly`, `-VcpkgPath D:\vcpkg`.

### Linux / macOS

Built in CI (`.github/workflows/build-linux.yml`, `build-macos.yml`), runnable
manually from the Actions tab. They build `libt2piv` and `libfido2`, the Rust
`fido2-manage` CLI and the GUI, bundle the shared libraries, and produce the
packages above.

## Repository layout

- `token2-piv-gui/`  — one Rust crate producing **`fido2-manage`** (the CLI) and
  the `token2-key-manager` GUI, sharing the FIDO/PIV code; links `libt2piv` and
  `libfido2`.
- `token2-piv-tool/` — the PIV CLI + `libt2piv` + `libt2cs11` (PKCS#11).
- `deps/libfido2/`   — cloned by the build from the upstream libfido2 at a pinned
  tag (default 1.15.0), built with `USE_PCSC=ON` and the Token2 CCID-tunnel patch
  (the Token2 FIDO applet is reached over the OTP applet's CCID channel).
- `build-all.ps1`    — Windows build driver (`build-all.cmd` wraps it).
- `.github/workflows/` — manual CI for Windows / Linux / macOS packages.

## About / licensing

Token2 Sàrl is a Swiss manufacturer of FIDO2 / FIDO U2F hardware tokens and
programmable TOTP devices — https://www.token2.com

Licensed under the **BSD 2-Clause** license. Bundled open-source components retain
their own licenses, listed in the application's **About** page.
