# Known issues

Workarounds for issues that are specific to a platform, reader or environment
rather than bugs in fido2-manage / Token2 Key Manager itself. Each entry links
the tracking issue where there is one.

## Token2 Key Manager (GUI)

### Certificate management is experimental

**Status.** The Certificates tab (key generation, self-signed certificates,
import/export, PFX/P12 import) is marked **experimental** in the UI. Some issues
have been reported and are still being worked on, so results on this tab may be
unreliable for now.

**Workaround.** For production certificate work, use the Companion App
until this is stabilised. These issues are planned to be fixed in a later
release.

### "Tap to open" disconnects the card on Dell integrated readers (#67)

**Symptom.** On a Dell laptop with the integrated **ControlVault 3+** reader
(Broadcom 58200, `USB\VID_0A5C&PID_5865`), presenting a Token2 FIDO2 card and
clicking **Tap to open** immediately disconnects it — the GUI shows
*"Key removed"* and then reconnects, and no management action can be performed.
The same card, reader and operations work from the `fido2-manage` CLI.

**Cause.** A firmware/host-components defect in the Dell ControlVault module.
The Windows inbox *Usbccid Smartcard Reader (WUDF)* driver is unchanged before
and after the fix; only Dell's ControlVault firmware/host components differ. The
GUI runs a periodic card-presence poll, so at *Tap to open* it briefly touches
the reader while also connecting — a conformant reader handles this, and the
affected ControlVault firmware did not until it was updated. The CLI connects
once with no background polling, so it never hits the contention.

**Fix / workaround.**
- Install the **Dell ControlVault 3+ Driver and Firmware** package from
  https://www.dell.com/support/home/ (Dell ControlVault Host Components
  Installer). No reboot was needed in the reported case. After this the GUI
  works with the integrated reader.
- Or use an external reader such as the
  [Token2 Compact Dual Reader (USB, NFC + Smart Card)](https://www.token2.com/products/token2-t2sr-compact-usb-nfc-and-smart-card-reader),
  which is unaffected.

### "We couldn't create the data directory" when run elevated as a different admin account (#66)

**Symptom.** On Windows, launching the GUI **elevated under a *different* admin
account** (over-the-shoulder UAC, `runas /user:`, LAPS) fails to start with:

    We couldn't create the data directory
    Microsoft Edge can't read and write to its data directory:
    C:\Users\<account>\AppData\Local\com.token2.pivmanager\EBWebView

Running non-elevated, or elevated as the interactively logged-on user, works.

**Cause.** WebView2 stores its data under `%LOCALAPPDATA%\com.token2.pivmanager`
of the account the process runs as, and cannot initialise that folder when the
app is elevated into a separate account's profile. The `WEBVIEW2_*` environment
variables are ignored for elevated hosts and there is no registry override for
the data folder.

**Fix / workaround.** Point the WebView2 data folder at a path the elevated
account can write, with either the command-line argument or the environment
variable (the argument wins; when neither is set the default per-user location
is used):

    token2-key-manager.exe --webview-data-dir C:\ProgramData\Token2\KeyManager\WebView2

    set TOKEN2_WEBVIEW_DATA_DIR=C:\ProgramData\Token2\KeyManager\WebView2
    token2-key-manager.exe

### AppImage crash on NVIDIA / Wayland

**Symptom.** On some NVIDIA + Wayland systems the Linux build can crash on
startup with an "Explicit Sync" / DMA-BUF renderer error.

**Fix / workaround.** The packaged launcher disables the WebKit DMA-BUF renderer
automatically when an NVIDIA GPU is detected. If a crash still occurs, launch
with the renderer disabled explicitly:

    WEBKIT_DISABLE_DMABUF_RENDERER=1 ./Token2-Key-Manager.AppImage

## fido2-manage (CLI)

### Factory reset only works right after plug-in

`-reset` is only accepted during the CTAP power-up window. Unplug and replug the
key, run the command, and touch it when it blinks. This is a FIDO2 protocol
requirement, not a tool limitation.
