//! Safe wrapper around libt2piv. One `Piv` = one open PC/SC session.

use crate::t2piv::*;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::ptr;

pub struct Piv {
    state: *mut t2piv_state,
}

// libt2piv state is only ever touched under the app-level Mutex.
unsafe impl Send for Piv {}

pub type Result<T> = std::result::Result<T, String>;

fn rc_to_result(rc: t2piv_rc) -> Result<()> {
    if rc == T2PIV_OK {
        Ok(())
    } else {
        let msg = unsafe { CStr::from_ptr(t2piv_strerror(rc)) }.to_string_lossy().into_owned();
        Err(format!("{msg} ({rc})"))
    }
}

/// A fresh random Windows container GUID, lowercase 8-4-4-4-12, for the 5FFF04
/// CONTAINER_MAP_RECORD.wszGuid field (the minidriver keys containers by this).
fn fresh_container_guid() -> String {
    let b: [u8; 16] = rand::random();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

#[derive(Serialize, Clone, Debug)]
pub struct KeyInfo {
    pub reader: String,
    pub version: String,
    pub serial: u32,
    pub serial_full: String,
    pub model: Option<crate::t2model::T2Model>,
    pub pin_retries: i32,
    pub puk_blocked: bool,
    pub mgm_type: String,
    pub mgm_algo: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SlotMeta {
    pub algorithm: u8,
    pub algorithm_name: String,
    pub pin_policy: u8,
    pub touch_policy: u8,
    pub origin: u8,
    pub public_key: Vec<u8>,
}

pub fn algo_name(a: u8) -> &'static str {
    match a {
        ALGO_RSA1024 => "RSA 1024",
        ALGO_RSA2048 => "RSA 2048",
        ALGO_RSA3072 => "RSA 3072",
        ALGO_RSA4096 => "RSA 4096",
        ALGO_ECCP256 => "ECC P-256",
        ALGO_ECCP384 => "ECC P-384",
        ALGO_ED25519 => "Ed25519",
        ALGO_X25519 => "X25519",
        ALGO_3DES => "3DES",
        ALGO_AES128 => "AES-128",
        ALGO_AES192 => "AES-192",
        ALGO_AES256 => "AES-256",
        _ => "",
    }
}

/// Algorithms the applet advertises in its SELECT response (tag AC/80 entries).
/// Returns raw algorithm IDs; asymmetric ones only.
impl Piv {
    pub fn supported_algorithms(&self) -> Result<Vec<u8>> {
        let templ = [0x00u8, 0xa4, 0x04, 0x00];
        let aid = [0xa0u8, 0x00, 0x00, 0x03, 0x08];
        let mut out = vec![0u8; 1024];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        rc_to_result(unsafe {
            t2piv_transfer_data(self.state, templ.as_ptr(), aid.as_ptr(), aid.len() as _, out.as_mut_ptr(), &mut out_len, &mut sw)
        })?;
        if sw != 0x9000 && sw >> 8 != 0x61 {
            return Err(format!("SELECT PIV failed: {sw:04x}"));
        }
        out.truncate(out_len as usize);
        // scan for AC 06 80 01 <alg> 06 01 00 sequences
        let mut ids = Vec::new();
        let mut i = 0;
        while i + 4 < out.len() {
            if out[i] == 0xac && out[i + 2] == 0x80 && out[i + 3] == 0x01 {
                let alg = out[i + 4];
                if !ids.contains(&alg) && !matches!(alg, ALGO_3DES | ALGO_AES128 | ALGO_AES192 | ALGO_AES256) {
                    ids.push(alg);
                }
                i += 2 + out[i + 1] as usize;
            } else {
                i += 1;
            }
        }
        Ok(ids)
    }
}

pub fn algo_from_name(s: &str) -> Option<u8> {
    if let Some(hex) = s.strip_prefix("0x") {
        return u8::from_str_radix(hex, 16).ok();
    }
    Some(match s {
        "RSA1024" => ALGO_RSA1024,
        "RSA2048" => ALGO_RSA2048,
        "RSA3072" => ALGO_RSA3072,
        "RSA4096" => ALGO_RSA4096,
        "ECCP256" => ALGO_ECCP256,
        "ECCP384" => ALGO_ECCP384,
        "ED25519" => ALGO_ED25519,
        "X25519" => ALGO_X25519,
        _ => return None,
    })
}

impl Piv {
    pub fn new() -> Result<Self> {
        let mut state: *mut t2piv_state = ptr::null_mut();
        rc_to_result(unsafe { t2piv_init(&mut state, 0) })?;
        Ok(Piv { state })
    }

    /// Reader names known to PC/SC.
    pub fn list_readers(&self) -> Result<Vec<String>> {
        let mut buf = vec![0u8; 4096];
        let mut len = buf.len();
        rc_to_result(unsafe { t2piv_list_readers(self.state, buf.as_mut_ptr() as *mut _, &mut len) })?;
        buf.truncate(len);
        Ok(buf
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect())
    }

    pub fn connect(&self, wanted: &str) -> Result<()> {
        let c = CString::new(wanted).map_err(|e| e.to_string())?;
        rc_to_result(unsafe { t2piv_connect(self.state, c.as_ptr()) })
    }

    /// Re-SELECT the PIV applet. A shared single-channel card may have had
    /// another applet (OTP/OATH/OpenPGP) selected by a capability probe or the
    /// FIDO tunnel; without re-selecting, the next PIN verify is rejected by the
    /// wrong applet and libt2piv returns AUTHENTICATION_ERROR (-5) with no retry
    /// count. Cheap and idempotent, so we do it before every PIV operation.
    #[allow(dead_code)]
    pub fn reselect(&self) -> Result<()> {
        let templ = [0x00u8, 0xa4, 0x04, 0x00];
        let aid = [0xa0u8, 0x00, 0x00, 0x03, 0x08];
        let mut out = vec![0u8; 512];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        rc_to_result(unsafe {
            t2piv_transfer_data(self.state, templ.as_ptr(), aid.as_ptr(), aid.len() as _, out.as_mut_ptr(), &mut out_len, &mut sw)
        })?;
        if sw != 0x9000 && sw >> 8 != 0x61 { return Err(format!("re-select PIV failed: {sw:04x}")); }
        Ok(())
    }

    pub fn disconnect(&self) {
        unsafe {
            t2piv_disconnect(self.state);
        }
    }

    pub fn version(&self) -> Result<String> {
        let mut buf = [0u8; 32];
        rc_to_result(unsafe { t2piv_get_version(self.state, buf.as_mut_ptr() as *mut _, buf.len()) })?;
        Ok(CStr::from_bytes_until_nul(&buf).map_err(|e| e.to_string())?.to_string_lossy().into_owned())
    }

    pub fn serial(&self) -> Result<u32> {
        let mut s = 0u32;
        rc_to_result(unsafe { t2piv_get_serial(self.state, &mut s) })?;
        Ok(s)
    }

    /// Full serial as printed on the key (OTP applet); empty if unavailable.
    pub fn serial_full(&self) -> String {
        let mut buf = [0u8; 32];
        let rc = unsafe { t2piv_get_serial_str(self.state, buf.as_mut_ptr() as *mut _, buf.len()) };
        if rc != T2PIV_OK {
            return String::new();
        }
        CStr::from_bytes_until_nul(&buf).map(|c| c.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn pin_retries(&self) -> Result<i32> {
        let mut t: c_int = 0;
        rc_to_result(unsafe { t2piv_get_pin_retries(self.state, &mut t) })?;
        Ok(t)
    }

    pub fn config(&self) -> Result<t2piv_config> {
        let mut cfg = t2piv_config::default();
        rc_to_result(unsafe { t2piv_util_get_config(self.state, &mut cfg) })?;
        Ok(cfg)
    }

    pub fn info(&self, reader: &str) -> Result<KeyInfo> {
        let cfg = self.config().unwrap_or_default();
        let mgm_meta = self.slot_metadata(0x9b).ok();
        Ok(KeyInfo {
            reader: reader.to_string(),
            version: self.version()?,
            serial: self.serial().unwrap_or(0),
            serial_full: self.serial_full(),
            model: crate::t2model::from_serial(&self.serial_full()),
            pin_retries: self.pin_retries().unwrap_or(-1),
            puk_blocked: cfg.puk_blocked != 0,
            mgm_type: match cfg.mgm_type {
                0 => "manual",
                1 => "derived",
                2 => "protected",
                _ => "unknown",
            }
            .into(),
            mgm_algo: mgm_meta.map(|m| m.algorithm_name).unwrap_or_default(),
        })
    }

    /// Returns tries left on failure so the UI can show it.
    /// Bind (enrol) the on-card fingerprint to biometric PIV auth, mirroring the
    /// Token2 Companion app: PIV `VERIFY` to key reference 0x96 (OCC / on-card
    /// comparison, the YubiKey-Bio-style reference) with a TLV carrying the
    /// current PIN — `00 20 00 96 <Lc> 03 <len> 01 <pin-ascii> FF FF`.
    /// Verified against a USBPcap of the Companion app doing this over CCID,
    /// non-elevated. `enable=false` clears the binding via VERIFY P1=0xFF.
    pub fn set_fingerprint_binding(&self, pin: &str, enable: bool) -> Result<()> {
        // 00 20 00 96 0b 03 09 <01=enable|00=disable> <PIN ascii> ff ff
        // (from a USBPcap of the Token2 Companion app toggling on and off).
        let templ = [0x00u8, 0x20, 0x00, 0x96];
        let mut inner = vec![if enable { 0x01u8 } else { 0x00 }];
        inner.extend_from_slice(pin.as_bytes());
        inner.extend_from_slice(&[0xff, 0xff]);
        let mut data = vec![0x03u8, inner.len() as u8];
        data.extend_from_slice(&inner);
        let mut out = vec![0u8; 64];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        rc_to_result(unsafe {
            t2piv_transfer_data(self.state, templ.as_ptr(), data.as_ptr(), data.len() as _, out.as_mut_ptr(), &mut out_len, &mut sw)
        })?;
        match sw {
            0x9000 | 0x6100..=0x61ff => {
                // Confirm the change took effect. On enable, the biometric ref
                // must now report as usable; if not, no fingerprint is enrolled.
                if enable && self.fingerprint_retries().is_none() {
                    return Err("enable returned OK but no fingerprint is enrolled — enroll a fingerprint on the FIDO side first (FIDO → Fingerprints), then enable PIV fingerprint auth".into());
                }
                Ok(())
            }
            0x6a80 | 0x6a88 => Err("this key does not support fingerprint-bound PIV".into()),
            0x6982 | 0x63c0..=0x63cf => Err("wrong PIN".into()),
            0x6983 => Err("PIN is blocked".into()),
            _ => Err(format!("fingerprint {}: status 0x{sw:04X}", if enable { "enable" } else { "disable" })),
        }
    }

    /// Verify by fingerprint (OCC reference 0x96, Tlv(3) empty): prompts the
    /// sensor; on a match the PIV security status is satisfied like a PIN verify.
    #[allow(dead_code)]
    pub fn verify_fingerprint(&self) -> Result<()> {
        let templ = [0x00u8, 0x20, 0x00, 0x96];
        let data = [0x03u8, 0x00];
        let mut out = vec![0u8; 64];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        let _ = unsafe {
            t2piv_transfer_data(self.state, templ.as_ptr(), data.as_ptr(), data.len() as _, out.as_mut_ptr(), &mut out_len, &mut sw)
        };
        match sw {
            0x9000 => Ok(()),
            s if s >> 8 == 0x61 => Ok(()),
            0x6a80 | 0x6a88 => Err("biometric PIV is not enabled on this key".into()),
            s if s & 0xfff0 == 0x63c0 => Err(format!("fingerprint did not match — {} attempt(s) left", s & 0x0f)),
            0x6983 => Err("biometric is blocked — use the PIN".into()),
            _ => Err(format!("fingerprint verify failed: {sw:04x}")),
        }
    }

    /// Remaining biometric attempts (VERIFY 0x96 empty = check-only). None => not bound.
    pub fn fingerprint_retries(&self) -> Option<u8> {
        let templ = [0x00u8, 0x20, 0x00, 0x96];
        let mut out = vec![0u8; 16];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        let _ = unsafe { t2piv_transfer_data(self.state, templ.as_ptr(), std::ptr::null(), 0, out.as_mut_ptr(), &mut out_len, &mut sw) };
        match sw {
            0x9000 => Some(0xff),
            s if s >> 8 == 0x61 => Some(0xff),
            s if s & 0xfff0 == 0x63c0 => Some((s & 0x0f) as u8),
            _ => None,
        }
    }

    /// Read whether biometric PIV auth is currently set: VERIFY 0x96 with empty
    /// data returns the retry/status word (6Cxx / 63Cx tries, or 9000 if verified).
    pub fn fingerprint_binding_status(&self) -> Result<bool> {
        // Vendor GET for the biometric 0x96 reference: response "01 01 XX",
        // last byte XX = 01 enabled / 00 disabled. (From a Companion-app USBPcap.)
        let templ = [0x80u8, 0xf7, 0x00, 0x96];
        let mut out = vec![0u8; 32];
        let mut out_len: std::os::raw::c_ulong = out.len() as _;
        let mut sw: c_int = 0;
        let _ = unsafe { t2piv_transfer_data(self.state, templ.as_ptr(), std::ptr::null(), 0, out.as_mut_ptr(), &mut out_len, &mut sw) };
        if sw == 0x9000 || sw >> 8 == 0x61 {
            let d = &out[..out_len as usize];
            // expect 01 01 XX
            if d.len() >= 3 && d[0] == 0x01 && d[1] == 0x01 {
                return Ok(d[2] != 0);
            }
        }
        Ok(false)
    }

    /// Whether the biometric-PIV reference exists at all (key supports the feature).
    pub fn fingerprint_binding_supported(&self) -> bool {
        // Only Bio keys have a fingerprint sensor. The 0x96 biometric reference also
        // answers on non-bio PIN+ PIV+ keys, so VERIFY 00 20 00 96 alone is not a
        // reliable discriminator (it would offer "Fingerprint for PIV login" on a key
        // with no sensor). Gate on the model's bio capability, derived from the serial.
        let is_bio = crate::t2model::from_serial(&self.serial_full()).map(|m| m.bio).unwrap_or(false);
        is_bio && self.fingerprint_retries().is_some()
    }

    /// Authorise a PIV operation: if `pin` is empty and biometric is available,
    /// verify by fingerprint (prompts the sensor); otherwise verify by PIN.
    #[allow(dead_code)] // kept for the (currently disabled) fingerprint-for-operations path
    #[allow(dead_code)]
    pub fn verify_for_op(&self, pin: &str) -> Result<()> {
        // Empty PIN means the UI asked for fingerprint auth (the user ticked the
        // box, only shown when biometric is available). Go straight to the
        // sensor verify — no speculative 0x96 probe that could disturb state.
        if pin.is_empty() {
            return self.verify_fingerprint();
        }
        self.verify_pin(pin)
    }

    pub fn verify_pin(&self, pin: &str) -> Result<()> {
        let c = CString::new(pin).map_err(|e| e.to_string())?;
        let mut tries: c_int = -1;
        let rc = unsafe { t2piv_verify(self.state, c.as_ptr(), &mut tries) };
        rc_to_result(rc).map_err(|e| if tries >= 0 { format!("{e}; {tries} tries left") } else { e })
    }

    pub fn change_pin(&self, current: &str, new: &str) -> Result<()> {
        let (c, n) = (CString::new(current).unwrap_or_default(), CString::new(new).unwrap_or_default());
        let mut tries: c_int = -1;
        let rc = unsafe {
            t2piv_change_pin(self.state, c.as_ptr(), current.len(), n.as_ptr(), new.len(), &mut tries)
        };
        rc_to_result(rc).map_err(|e| if tries >= 0 { format!("{e}; {tries} tries left") } else { e })
    }

    pub fn change_puk(&self, current: &str, new: &str) -> Result<()> {
        let (c, n) = (CString::new(current).unwrap_or_default(), CString::new(new).unwrap_or_default());
        let mut tries: c_int = -1;
        let rc = unsafe {
            t2piv_change_puk(self.state, c.as_ptr(), current.len(), n.as_ptr(), new.len(), &mut tries)
        };
        rc_to_result(rc).map_err(|e| if tries >= 0 { format!("{e}; {tries} tries left") } else { e })
    }

    pub fn unblock_pin(&self, puk: &str, new_pin: &str) -> Result<()> {
        let (p, n) = (CString::new(puk).unwrap_or_default(), CString::new(new_pin).unwrap_or_default());
        let mut tries: c_int = -1;
        let rc = unsafe {
            t2piv_unblock_pin(self.state, p.as_ptr(), puk.len(), n.as_ptr(), new_pin.len(), &mut tries)
        };
        rc_to_result(rc).map_err(|e| if tries >= 0 { format!("{e}; {tries} tries left") } else { e })
    }

    pub fn set_pin_retries(&self, pin: i32, puk: i32) -> Result<()> {
        rc_to_result(unsafe { t2piv_set_pin_retries(self.state, pin, puk) })
    }

    pub fn authenticate(&self, key: &[u8]) -> Result<()> {
        rc_to_result(unsafe { t2piv_authenticate2(self.state, key.as_ptr(), key.len()) })
    }

    pub fn set_mgm_key(&self, key: &[u8], algorithm: u8, touch: u8) -> Result<()> {
        rc_to_result(unsafe { t2piv_set_mgmkey3(self.state, key.as_ptr(), key.len(), algorithm, touch) })
    }

    pub fn slot_metadata(&self, slot: u8) -> Result<SlotMeta> {
        let mut buf = vec![0u8; 2048];
        let mut len = buf.len();
        rc_to_result(unsafe { t2piv_get_metadata(self.state, slot, buf.as_mut_ptr(), &mut len) })?;
        let mut md = t2piv_metadata::default();
        rc_to_result(unsafe { t2piv_util_parse_metadata(buf.as_mut_ptr(), len, &mut md) })?;
        Ok(SlotMeta {
            algorithm: md.algorithm,
            algorithm_name: algo_name(md.algorithm).to_string(),
            pin_policy: md.pin_policy,
            touch_policy: md.touch_policy,
            origin: md.origin,
            public_key: md.pubkey[..md.pubkey_len.min(1024)].to_vec(),
        })
    }

    pub fn read_cert(&self, slot: u8) -> Result<Option<Vec<u8>>> {
        let mut data: *mut u8 = ptr::null_mut();
        let mut len: usize = 0;
        let rc = unsafe { t2piv_util_read_cert(self.state, slot, &mut data, &mut len) };
        if rc != T2PIV_OK || data.is_null() || len == 0 {
            return Ok(None);
        }
        let v = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
        unsafe { t2piv_util_free(self.state, data as *mut c_void) };
        Ok(Some(v))
    }

    pub fn write_cert(&self, slot: u8, der: &[u8]) -> Result<()> {
        let mut buf = der.to_vec();
        rc_to_result(unsafe { t2piv_util_write_cert(self.state, slot, buf.as_mut_ptr(), buf.len(), 0) })
    }

    pub fn delete_cert(&self, slot: u8) -> Result<()> {
        rc_to_result(unsafe { t2piv_util_delete_cert(self.state, slot) })
    }

    /// Delete the private key in a slot. Prefers the vendor 00 EE command (works
    /// with the user PIN alone on Token2 firmware, as the Companion app uses);
    /// falls back to move-key (needs applet 5.7.0+). The slot is passed as P1.
    /// Verified against a USBPcap of the Companion app: `00 EE <slot> 00`.
    pub fn delete_key(&self, slot: u8) -> Result<()> {
        // Token2 private-key delete, PIN-only, reproduced from a USBPcap of the
        // Companion app (slots 9a and 9c). The user PIN must already be verified.
        // Sequence: prepare (00 EE 01) -> clear the slot's cert + three per-slot
        // metadata objects -> 00 FE 00 <slot> (the actual key delete) -> best-effort
        // cleanup of the minidriver container map (5FFF04 / 5FFF03). The key is
        // gone after 00 FE returns 9000; the map writes are bookkeeping.
        let ci = match slot {          // container index (creation order)
            0x9a => 0, 0x9c => 1, 0x9d => 2, 0x9e => 3,
            0x82..=0x95 => 4 + (slot - 0x82),
            _ => return Err(format!("delete not supported for slot {slot:02x}")),
        };
        let cert_obj: [u8;3] = match slot {   // PIV cert object per slot
            0x9a => [0x5f,0xc1,0x05], 0x9c => [0x5f,0xc1,0x0a],
            0x9d => [0x5f,0xc1,0x0b], 0x9e => [0x5f,0xc1,0x01],
            _ => [0x5f,0xc1,0x05],
        };
        let send = |templ: &[u8;4], data: &[u8]| -> (i32, u16) {
            let mut out = vec![0u8; 1024];
            let mut ol: std::os::raw::c_ulong = out.len() as _;
            let mut sw: c_int = 0;
            let rc = unsafe { t2piv_transfer_data(self.state, templ.as_ptr(),
                if data.is_empty() { std::ptr::null() } else { data.as_ptr() },
                data.len() as _, out.as_mut_ptr(), &mut ol, &mut sw) };
            (rc, sw as u16)
        };
        let put_clear = |obj: [u8;3]| {
            let d = [0x5cu8, 0x03, obj[0], obj[1], obj[2], 0x53, 0x00];
            let _ = send(&[0x00,0xdb,0x3f,0xff], &d);
        };
        // 1) prepare
        let _ = send(&[0x00, 0xee, 0x01, 0x00], &[]);
        // 2) clear the cert object + three per-slot metadata objects
        put_clear(cert_obj);
        put_clear([0x5f, 0xff, 0x28 + ci]);
        put_clear([0x5f, 0xff, 0x10 + ci]);
        put_clear([0x5f, 0xff, 0x50 + ci]);
        // 3) the actual key delete
        let (_r, sw) = send(&[0x00, 0xfe, 0x00, slot], &[]);
        if sw != 0x9000 && sw >> 8 != 0x61 {
            return Err(format!("the applet refused the key delete (status {sw:04X}). Overwrite the slot with a new key instead."));
        }
        // 4) best-effort: remove this slot's entry from the minidriver container map
        //    (5FFF04). Read the current map, drop the 112-byte entry ending in
        //    <slot> {16|07} 03, and write it back. Failure here doesn't matter —
        //    the key is already gone.
        let _ = self.rewrite_container_map(slot);
        // 5) verify the key is actually gone
        let still = self.slot_metadata(slot).ok().map(|m| !m.algorithm_name.is_empty()).unwrap_or(false);
        if still { return Err("the applet accepted the delete but the key still reads as present".into()); }
        Ok(())
    }

    /// Best-effort removal of one slot's entry from the 5FFF04 container map.
    fn rewrite_container_map(&self, slot: u8) -> Result<()> {
        let send = |templ: &[u8;4], data: &[u8], out: &mut [u8]| -> (u16, usize) {
            let mut ol: std::os::raw::c_ulong = out.len() as _;
            let mut sw: c_int = 0;
            let _ = unsafe { t2piv_transfer_data(self.state, templ.as_ptr(),
                if data.is_empty() { std::ptr::null() } else { data.as_ptr() },
                data.len() as _, out.as_mut_ptr(), &mut ol, &mut sw) };
            (sw as u16, ol as usize)
        };
        let mut buf = vec![0u8; 2048];
        let (sw, n) = send(&[0x00,0xcb,0x3f,0xff], &[0x5c,0x03,0x5f,0xff,0x04,0x00], &mut buf);
        if sw != 0x9000 && sw >> 8 != 0x61 { return Ok(()); } // no map / can't read: leave it
        let resp = &buf[..n];
        // strip 53 <len> to the value
        let Some(t) = resp.iter().position(|&b| b == 0x53) else { return Ok(()) };
        let after = &resp[t+1..];
        let (voff, _vlen) = if after.first()==Some(&0x82) { (t+4, ((after[1] as usize)<<8)|after[2] as usize) }
            else if after.first()==Some(&0x81) { (t+3, after[1] as usize) }
            else { (t+2, *after.first().unwrap_or(&0) as usize) };
        let mut value = resp[voff..].to_vec();
        // locate the 112-byte entry whose trailer is <slot> {16|07} 03 and zero it
        let mut i = 0usize; let mut zeroed = false;
        while i + 3 <= value.len() {
            if value[i]==slot && (value[i+1]==0x16 || value[i+1]==0x07) && value[i+2]==0x03 {
                let end = i + 2; let start = end.saturating_sub(111);
                for b in &mut value[start..=end] { *b = 0; }
                zeroed = true; break;
            }
            i += 1;
        }
        if !zeroed { return Ok(()); }
        // write the map back with the same outer framing
        let mut put = vec![0x5cu8, 0x03, 0x5f, 0xff, 0x04];
        put.extend_from_slice(&resp[t..t+ (voff - t)]); // 53 <len> header
        put.extend_from_slice(&value);
        let mut o = vec![0u8; 16];
        let _ = send(&[0x00,0xdb,0x3f,0xff], &put, &mut o);
        Ok(())
    }

    /// Provision the Windows PIV+ minidriver container objects for a key that was
    /// just generated or imported into `slot`. Reproduced from a USBPcap of the
    /// Companion app (slot 9c, RSA-2048): the Token2 PIV+ minidriver does NOT read
    /// the standard CHUID (5FC102) / CCC (5FC107). It enumerates containers from two
    /// proprietary objects, so without them a key is present but "cannot be opened"
    /// on Windows (certutil -SCInfo shows `(null)`):
    ///   5FFF04  - container map: an array of 112-byte Windows CONTAINER_MAP_RECORDs
    ///   5FFF03  - per-slot key-info object
    /// We read-modify-write 5FFF04 (so any containers the Companion already wrote stay
    /// intact), then write 5FFF03, mirroring the Companion byte-for-byte. The three
    /// stale per-slot metadata objects (5FFF10/28/50 + container-index) are cleared
    /// first, exactly as the Companion does before writing the new entry.
    ///
    /// The management key must already be authenticated on `self`.
    ///
    /// NOTE: only RSA-2048 is capture-verified. Other algorithms use the same record
    /// layout with their PIV algorithm code and key size; that path still needs
    /// hardware validation, which is why PIV stays experimental.
    pub fn provision_container(&self, slot: u8, algorithm: u8, admin_key: &[u8]) -> Result<()> {
        // Re-assert management auth: generating/importing a key clears the management
        // security status on this applet, and the 5FFFxx PUT DATA writes need it. Runs
        // in the same session as generate/import (not a second session, which would
        // re-verify the PIN and fail on this applet right after a key operation).
        // Best-effort: if auth was NOT cleared, re-authenticating can itself report
        // "not supported" (already authenticated) - harmless, so ignore it and let the
        // PUT DATA writes below surface any genuine security-status error (SW 6982).
        let _ = self.authenticate(admin_key);
        // Keep the standard CHUID/CCC as well - harmless, and generic PIV middleware
        // (OpenSC and friends) still reads them even though the minidriver does not.
        let _ = self.ensure_card_identity();

        let ci: u8 = match slot {          // container index (creation order)
            0x9a => 0, 0x9c => 1, 0x9d => 2, 0x9e => 3,
            0x82..=0x95 => 4 + (slot - 0x82),
            _ => return Err(format!("minidriver provisioning not supported for slot {slot:02x}")),
        };
        let key_bits: u16 = match algorithm {
            ALGO_RSA1024 => 1024, ALGO_RSA2048 => 2048,
            ALGO_RSA3072 => 3072, ALGO_RSA4096 => 4096,
            ALGO_ECCP384 => 384,
            ALGO_ECCP256 | ALGO_ED25519 | ALGO_X25519 => 256,
            _ => 2048,
        };

        let send = |templ: &[u8; 4], data: &[u8]| -> (u16, Vec<u8>) {
            let mut out = vec![0u8; 4096];
            let mut ol: std::os::raw::c_ulong = out.len() as _;
            let mut sw: c_int = 0;
            let _ = unsafe {
                t2piv_transfer_data(self.state, templ.as_ptr(),
                    if data.is_empty() { ptr::null() } else { data.as_ptr() },
                    data.len() as _, out.as_mut_ptr(), &mut ol, &mut sw)
            };
            let n = (ol as usize).min(out.len());
            out.truncate(n);
            (sw as u16, out)
        };
        let put = |obj: [u8; 3], val: &[u8]| -> u16 {
            let mut d = vec![0x5cu8, 0x03, obj[0], obj[1], obj[2]];
            d.extend_from_slice(val);
            send(&[0x00, 0xdb, 0x3f, 0xff], &d).0
        };

        // 1) Clear the three stale per-slot metadata objects for this container index.
        let _ = put([0x5f, 0xff, 0x10 + ci], &[0x53, 0x00]);
        let _ = put([0x5f, 0xff, 0x28 + ci], &[0x53, 0x00]);
        let _ = put([0x5f, 0xff, 0x50 + ci], &[0x53, 0x00]);

        // 2) Read the current container map (5FFF04), if any, down to its record array.
        const REC: usize = 112;
        let mut records: Vec<u8> = Vec::new();
        let (sw, buf) = send(&[0x00, 0xcb, 0x3f, 0xff], &[0x5c, 0x03, 0x5f, 0xff, 0x04, 0x00]);
        if sw == 0x9000 || sw >> 8 == 0x61 {
            if let Some(t) = buf.iter().position(|&b| b == 0x53) {
                let after = &buf[t + 1..];
                let voff = match after.first() {
                    Some(&0x82) => t + 4,
                    Some(&0x81) => t + 3,
                    _ => t + 2,
                };
                let value = buf.get(voff..).unwrap_or(&[]);
                // value is wrapped once more: 81 82 <len16> <records...>
                records = if value.first() == Some(&0x81) && value.get(1) == Some(&0x82) {
                    value.get(4..).unwrap_or(&[]).to_vec()
                } else {
                    value.to_vec()
                };
            }
        }
        let keep = (records.len() / REC) * REC; // only whole records
        records.truncate(keep);

        // 3) Assign a key index: reuse this slot's existing entry index, else max+1.
        //    Trailer layout at offset 100: <slot> <algo> <sizecode> 00 <keyidx> c1 5f ...
        let mut key_idx: u8 = 1;
        let mut existing: Option<usize> = None;
        let mut max_idx: u8 = 0;
        let mut off = 0usize;
        while off + REC <= records.len() {
            let rec = &records[off..off + REC];
            if rec[104] > max_idx { max_idx = rec[104]; }
            if rec[86] == slot { existing = Some(off); key_idx = rec[104]; }
            off += REC;
        }
        if existing.is_none() { key_idx = max_idx.wrapping_add(1).max(1); }

        // 4) Build the 112-byte CONTAINER_MAP_RECORD for this slot.
        let guid = fresh_container_guid();
        let mut rec = vec![0u8; REC];
        for (i, u) in guid.encode_utf16().enumerate() {   // wszGuid[40] UTF-16LE
            if i * 2 + 2 > 80 { break; }
            rec[i * 2] = (u & 0xff) as u8;
            rec[i * 2 + 1] = (u >> 8) as u8;
        }
        rec[80] = 0x01;                                   // valid flag
        rec[84] = (key_bits & 0xff) as u8;                // key size (LE u16)
        rec[85] = (key_bits >> 8) as u8;
        rec[86] = slot;
        rec[100] = slot;                                  // trailer
        rec[101] = algorithm;
        rec[102] = algorithm;                             // size-code (07 07 for RSA-2048)
        rec[103] = 0x00;
        rec[104] = key_idx;
        rec[105] = 0xc1;
        rec[106] = 0x5f;

        match existing {
            Some(o) => records[o..o + REC].copy_from_slice(&rec),
            None => records.extend_from_slice(&rec),
        }

        // 5) Write 5FFF04 back with the Companion's framing: 53 82 <len> 81 82 <len> <records>.
        //    libt2piv command-chains the >255-byte APDU automatically.
        let inner = records.len();
        let wrapped = inner + 4;
        let mut put04 = vec![
            0x5cu8, 0x03, 0x5f, 0xff, 0x04,
            0x53, 0x82, (wrapped >> 8) as u8, (wrapped & 0xff) as u8,
            0x81, 0x82, (inner >> 8) as u8, (inner & 0xff) as u8,
        ];
        put04.extend_from_slice(&records);
        let sw04 = send(&[0x00, 0xdb, 0x3f, 0xff], &put04).0;
        if sw04 != 0x9000 && sw04 >> 8 != 0x61 {
            return Err(format!("writing the Windows container map (5FFF04) failed (status {sw04:04X})."));
        }

        // 6) Write the per-slot key-info object 5FFF03.
        let v03 = [0x53u8, 0x08, 0x81, 0x06, 0x00, 0x01, key_idx, 0x00, 0x12, 0x00];
        let sw03 = put([0x5f, 0xff, 0x03], &v03);
        if sw03 != 0x9000 && sw03 >> 8 != 0x61 {
            return Err(format!("writing the Windows key-info object (5FFF03) failed (status {sw03:04X})."));
        }
        Ok(())
    }

    pub fn attest(&self, slot: u8) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 4096];
        let mut len = buf.len();
        rc_to_result(unsafe { t2piv_attest(self.state, slot, buf.as_mut_ptr(), &mut len) })?;
        buf.truncate(len);
        Ok(buf)
    }

    pub fn reset(&self) -> Result<()> {
        rc_to_result(unsafe { t2piv_util_reset(self.state) })
    }

    /// Ensure the card carries a CHUID and CCC, writing a fresh one only when
    /// missing (so the card GUID stays stable across key generations). Without
    /// these objects the Windows PIV minidriver cannot enumerate key containers
    /// and reports "the keys cannot be opened" even though the keys are present.
    /// The management key must already be authenticated on `self`.
    pub fn ensure_card_identity(&self) -> Result<()> {
        unsafe {
            // Treat a missing OR all-zero object as "not provisioned": get_cardid can
            // return success with an empty buffer, which would otherwise skip the write.
            let mut chuid = [0u8; 16];
            let have_chuid = rc_to_result(t2piv_util_get_cardid(self.state, chuid.as_mut_ptr())).is_ok()
                && chuid.iter().any(|&b| b != 0);
            if !have_chuid {
                rc_to_result(t2piv_util_set_cardid(self.state, ptr::null()))?;
            }
            let mut ccc = [0u8; 14];
            let have_ccc = rc_to_result(t2piv_util_get_cccid(self.state, ccc.as_mut_ptr())).is_ok()
                && ccc.iter().any(|&b| b != 0);
            if !have_ccc {
                rc_to_result(t2piv_util_set_cccid(self.state, ptr::null()))?;
            }
        }
        Ok(())
    }

    /// Returns the raw public key: (modulus, exponent) for RSA, point for EC/Ed/X.
    pub fn generate_key(&self, slot: u8, algorithm: u8, pin_policy: u8, touch_policy: u8) -> Result<GeneratedKey> {
        let (mut m, mut ml, mut e, mut el, mut p, mut pl): (*mut u8, usize, *mut u8, usize, *mut u8, usize) =
            (ptr::null_mut(), 0, ptr::null_mut(), 0, ptr::null_mut(), 0);
        rc_to_result(unsafe {
            t2piv_util_generate_key(
                self.state, slot, algorithm, pin_policy, touch_policy, &mut m, &mut ml, &mut e, &mut el, &mut p, &mut pl,
            )
        })?;
        let take = |ptr: *mut u8, len: usize| -> Vec<u8> {
            if ptr.is_null() || len == 0 {
                return vec![];
            }
            let v = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
            unsafe { t2piv_util_free(self.state, ptr as *mut c_void) };
            v
        };
        Ok(GeneratedKey { algorithm, modulus: take(m, ml), exponent: take(e, el), point: take(p, pl) })
    }

    /// Import a private key. RSA components must each be modulus_len/2 bytes,
    /// EC/Ed/X data the curve size.

    /// OpenSSL detail of the last PKCS#12 failure.
    pub fn import_last_error() -> String {
        let p = unsafe { t2piv_util_import_last_error() };
        if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
    }
    /// Parse-only check of a PKCS#12/PEM blob: (algorithm, has_cert).
    pub fn probe_key_blob(data: &[u8], password: &str) -> Result<(u8, bool)> {
        let pw = CString::new(password).map_err(|e| e.to_string())?;
        let mut algo: u8 = 0;
        let mut has_cert: c_int = 0;
        let rc = unsafe { t2piv_util_probe_key_blob(data.as_ptr(), data.len(), pw.as_ptr(), &mut algo, &mut has_cert) };
        if rc != T2PIV_OK {
            let detail = Self::import_last_error();
            return Err(match rc {
                -5 => format!("could not open the PKCS#12 file: {}", if detail.is_empty() { "wrong password or unsupported encryption".into() } else { detail }),
                -12 => format!("unsupported key type in file ({detail})"),
                _ => format!("unreadable file: {detail}"),
            });
        }
        Ok((algo, has_cert != 0))
    }

    /// Import a PKCS#12/PFX or PEM private key (+ cert) into a slot.
    /// Returns (algorithm, cert_written).
    pub fn import_key_blob(&self, slot: u8, data: &[u8], password: &str, pin_policy: u8, touch_policy: u8, write_cert: bool) -> Result<(u8, bool)> {
        let pw = CString::new(password).map_err(|e| e.to_string())?;
        let mut algo: u8 = 0;
        let mut written: c_int = 0;
        let rc = unsafe {
            t2piv_util_import_key_blob(self.state, slot, data.as_ptr(), data.len(), pw.as_ptr(), pin_policy, touch_policy, write_cert as c_int, &mut algo, &mut written)
        };
        match rc {
            T2PIV_OK => Ok((algo, written != 0)),
            -5 => Err("authentication failed (device refused the operation)".into()),
            -12 => Err("Unsupported key type (RSA with e=65537, P-256, P-384, Ed25519, X25519 only)".into()),
            -9 => Err("File is not a readable PKCS#12/PFX or PEM private key".into()),
            _ => rc_to_result(rc).map(|_| (algo, false)),
        }
    }

    /// Raw sign: input must already be hashed/padded as the algorithm expects.
    pub fn sign_raw(&self, slot: u8, algorithm: u8, input: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; 1024];
        let mut out_len = out.len();
        rc_to_result(unsafe {
            t2piv_sign_data(self.state, input.as_ptr(), input.len(), out.as_mut_ptr(), &mut out_len, algorithm, slot)
        })?;
        out.truncate(out_len);
        Ok(out)
    }
}

impl Drop for Piv {
    fn drop(&mut self) {
        unsafe {
            t2piv_done(self.state);
        }
    }
}

pub struct GeneratedKey {
    pub algorithm: u8,
    pub modulus: Vec<u8>,
    pub exponent: Vec<u8>,
    pub point: Vec<u8>,
}
