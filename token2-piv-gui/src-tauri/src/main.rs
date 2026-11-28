#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use token2_keymanager::commands::*;
use token2_keymanager::fido::*;
use token2_keymanager::oath::*;
use token2_keymanager::t2otp::*;
use std::sync::Mutex;

/// Optional override for the WebView2 (Windows) / WebKit (Linux) user-data folder.
///
/// Why: when the app is launched elevated under a *separate* admin account
/// (over-the-shoulder elevation, LAPS, `runas /user:`), WebView2 cannot create
/// its default data folder under that account's profile and the app fails with
/// "We couldn't create the data directory" (issue #66). WebView2 ignores the
/// `WEBVIEW2_*` environment variables for elevated hosts, and there is no
/// registry override for the data folder, so the path must be supplied here.
///
/// Priority:
///   1. `--webview-data-dir <path>`  or  `--webview-data-dir=<path>`  (CLI arg)
///   2. `TOKEN2_WEBVIEW_DATA_DIR`                                     (env var)
///
/// Point it at a location the elevated account can always write, e.g.
///   token2-key-manager.exe --webview-data-dir C:\ProgramData\Token2\KeyManager\WebView2
///
/// When neither is set this returns None and Tauri keeps its default
/// (`%LOCALAPPDATA%\com.token2.pivmanager`), so existing behaviour is unchanged.
fn webview_data_dir_override() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--webview-data-dir" {
            if let Some(val) = args.next() {
                if !val.is_empty() {
                    return Some(PathBuf::from(val));
                }
            }
        } else if let Some(val) = arg.strip_prefix("--webview-data-dir=") {
            if !val.is_empty() {
                return Some(PathBuf::from(val));
            }
        }
    }

    match std::env::var("TOKEN2_WEBVIEW_DATA_DIR") {
        Ok(val) if !val.is_empty() => Some(PathBuf::from(val)),
        _ => None,
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The main window is built here (not in tauri.conf.json) so we can set an
            // explicit WebView2 data directory. Tauri respects a data_directory we set
            // and otherwise forces %LOCALAPPDATA%\<identifier>, so leaving it unset keeps
            // the previous default.
            use tauri::{WebviewUrl, WebviewWindowBuilder};

            let title = format!("Token2 Key Manager {}", env!("CARGO_PKG_VERSION"));
            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title(title)
                    .inner_size(1080.0, 720.0)
                    .min_inner_size(900.0, 600.0);

            if let Some(dir) = webview_data_dir_override() {
                // Best-effort: create it so WebView2 gets an existing, writable folder.
                let _ = std::fs::create_dir_all(&dir);
                builder = builder.data_directory(dir);
            }

            builder.build()?;
            Ok(())
        })
        .manage(AppState(Mutex::new(Session::default())))
        .manage(FidoState(Mutex::new(FidoSession::default())))
        .manage(OathState(Mutex::new(OathSession::default())))
        .manage(T2OtpState(Mutex::new(T2OtpSession::default())))
        .invoke_handler(tauri::generate_handler![
            list_readers,
            connect,
            disconnect,
            get_info,
            list_slots,
            verify_pin,
            change_pin,
            change_puk,
            unblock_pin,
            unblock_pin_with_admin,
            set_retries,
            authenticate,
            set_mgm_key,
            supported_algorithms,
            generate_key,
            public_key_pem,
            self_sign,
            import_cert,
            import_pfx,
            pfx_pick,
            pfx_probe,
            export_cert,
            delete_cert,
            delete_slot,
            piv_supports_key_delete,
            piv_fp_binding_status,
            piv_default_admin_key,
            piv_set_fp_binding,
            attest,
            verify_attestation,
            attestation_chain,
            save_text,
            reset,
            exit_app,
            fido_list_devices,
            fido_connect,
            fido_connect_ccid,
            fido_environment,
            fido_disconnect,
            fido_info,
            fido_set_pin,
            fido_set_min_pin_len,
            fido_force_pin_change,
            fido_toggle_always_uv,
            fido_reset,
            fido_note_get,
            fido_note_set,
            fido_note_clear,
            fido_list_passkeys,
            fido_delete_passkey,
            fido_update_passkey,
            fido_list_bio,
            fido_rename_bio,
            fido_delete_bio,
            fido_enroll_begin,
            fido_enroll_continue,
            fido_enroll_cancel,
            fido_libfido2_version,
            fido_diagnose,
            key_capabilities,
            presence,
            cards_present,
            pcsc_health,
            pcsc_status,
            otp_hid_detect,
            otp_hid_debug,
            capabilities_debug,
            oath_list_readers,
            oath_connect,
            oath_connect_hid,
            t2otp_connect_hid,
            oath_disconnect,
            oath_unlock,
            oath_list,
            oath_calculate,
            oath_put,
            oath_delete,
            oath_set_password,
            oath_reset,
            oath_parse_uri,
            t2otp_connect,
            t2otp_disconnect,
            t2otp_config,
            t2otp_list,
            t2otp_read,
            t2otp_write,
            t2otp_delete,
            t2otp_erase_all,
            t2otp_enable_totp,
            t2otp_set_button_hotp,
            t2otp_delete_button_hotp,
            t2otp_set_button_options,
            t2otp_set_keyboard_interface,
            t2otp_pin_status,
            t2otp_verify_pin,
            t2otp_verify_fingerprint,
            t2otp_set_pin,
            t2otp_change_pin,
            t2otp_lock,
            t2otp_set_alpha_min_pin_len,
            t2otp_applet_mode,
            t2otp_device_info_raw,
            t2otp_applet_flags,
            t2otp_set_applet_flags,
            t2otp_applet_reset,
            qr_decode_image,
            qr_scan_screen,
            qr_scan_file,
            app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running token2-piv-gui");
}
