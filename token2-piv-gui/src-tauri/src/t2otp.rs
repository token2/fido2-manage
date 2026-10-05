//! Token2 OTP applet (T2F2 / PIN+ on-device TOTP/HOTP), per
//! token2-otp-cli/docs/Token2-OTP-SDK-Protocol.md. PC/SC transport only:
//! SELECT F0 00 00 01 4F 74 70 01 (short Lc), everything else extended Lc.
//! Seed writes are wrapped in ECDH(P-256) → SHA-256 → AES-256-CBC(PKCS#7)
//! with the two fixed IVs from the spec.

use aes::cipher::{block_padding::{NoPadding, Pkcs7}, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use p256::ecdh::EphemeralSecret;
use p256::{EncodedPoint, PublicKey};
use pcsc::{Card, Context, Disposition, Protocols, Scope, ShareMode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use tauri::State;

type R<T> = Result<T, String>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type HmacSha256 = Hmac<Sha256>;

const AID: [u8; 8] = [0xf0, 0x00, 0x00, 0x01, 0x4f, 0x74, 0x70, 0x01];
const IV_ENTRY: [u8; 16] = [0x9d, 0xd8, 0x91, 0x8e, 0x34, 0xf3, 0xcc, 0xab, 0x08, 0xcb, 0x75, 0x18, 0xf7, 0x19, 0x38, 0xf1];
const IV_BUTTON: [u8; 16] = [0; 16];

// CLA INS P1 P2
const WRITE_HOTP_SEED: [u8; 4] = [0x80, 0xc5, 0x00, 0x00];
const GET_ECDH_PUBKEY: [u8; 4] = [0x80, 0xc5, 0x01, 0x00];
const READ_CONFIG: [u8; 4] = [0x80, 0xc5, 0x02, 0x00];
const CFG_HOTP_ENTER: [u8; 4] = [0x80, 0xc5, 0x02, 0x02];
const CFG_HOTP_TOUCH: [u8; 4] = [0x80, 0xc5, 0x02, 0x04];
const SET_DEVICE_TYPE: [u8; 4] = [0x80, 0xc5, 0x02, 0x01];
const ENABLE_TOTP: [u8; 4] = [0x80, 0xc5, 0x02, 0x05];
const CFG_HOTP_KBD: [u8; 4] = [0x80, 0xc5, 0x02, 0x06];
const ENUM_CODES: [u8; 4] = [0x80, 0xc5, 0x05, 0x00];
const ENUM_CONTINUE: [u8; 4] = [0x80, 0xc5, 0x05, 0x01];
const WRITE_SEED: [u8; 4] = [0x80, 0xc5, 0x05, 0x02];
// OTP-PIN / privacy protection (R3.4+, manual V1.2) — ported from token2/T2TOTP_Authenticator
const READ_OTP_PIN_FLAG: [u8; 4] = [0x80, 0xc5, 0x05, 0x04];
const SET_OTP_PIN: [u8; 4] = [0x80, 0xc5, 0x05, 0x05];
const VERIFY_OTP_PIN: [u8; 4] = [0x80, 0xc5, 0x05, 0x06];
const CHANGE_OTP_PIN: [u8; 4] = [0x80, 0xc5, 0x05, 0x08];
const READ_AGREEMENT_PUBKEY: [u8; 4] = [0x80, 0xc5, 0x05, 0x09];
const PIN_ALG_AES256: u8 = 0x07;
const PIN_MAX_RETRY: u8 = 0x64;

pub struct SessionKeys {
    enc: [u8; 32],
    mac: [u8; 32],
}
/// The channel to the OTP applet: PC/SC card (default) or the CTAPHID tunnel for
/// HID-only Token2 keys (PIN+ Release2). APDUs are identical on both.
pub enum Wire {
    Pcsc(Card),
    Hid(crate::t2otp_hid::OtpHid),
}
impl Wire {
    fn xmit_once(&self, apdu: &[u8]) -> R<(Vec<u8>, u16)> {
        match self {
            Wire::Pcsc(card) => {
                let mut buf = vec![0u8; 65538];
                let r = card.transmit(apdu, &mut buf).map_err(|e| format!("PC/SC transmit: {e}"))?;
                if r.len() < 2 { return Err("short response".into()); }
                let sw = ((r[r.len()-2] as u16) << 8) | r[r.len()-1] as u16;
                Ok((r[..r.len()-2].to_vec(), sw))
            }
            Wire::Hid(hid) => hid.transmit(apdu),
        }
    }
    fn reset(&mut self) -> R<()> {
        if let Wire::Pcsc(card) = self {
            card.reconnect(pcsc::ShareMode::Shared, pcsc::Protocols::ANY, pcsc::Disposition::ResetCard)
                .map_err(|x| format!("card reset: {x}"))?;
        }
        Ok(())
    }
}

pub struct T2OtpSession {
    wire: Option<Wire>,
    reader: String,
    _ctx: Option<Context>,
    session: Option<SessionKeys>,
    pin_verified: bool,
}
impl Default for T2OtpSession {
    fn default() -> Self { Self { wire: None, reader: String::new(), _ctx: None, session: None, pin_verified: false } }
}
pub struct T2OtpState(pub Mutex<T2OtpSession>);

fn sw_err(what: &str, sw: u16) -> String {
    match sw {
        0x6a80 | 0x6a83 => format!("{what}: entry not found"),
        0x6a84 => format!("{what}: not enough space on the device"),
        0x6a86 => format!("{what}: HOTP-on-button is not supported on this model"),
        0x6ff9 => format!("{what}: timed out waiting for the button — touch the key and retry"),
        0x6a81 => format!("{what}: not allowed in the current PIN state"),
        0x6982 => format!("{what}: OTP PIN not verified or wrong PIN"),
        0x6983 => format!("{what}: OTP PIN is blocked after too many wrong attempts"),
        _ => format!("{what}: status 0x{sw:04X}"),
    }
}
/// APDU framing as validated by keyroost against T=0 contact readers, NFC and
/// HID: short Lc for bodies <= 255 bytes (T=0 readers reject extended Lc with
/// 6700), extended only above that, and a bare 4-byte header for an empty body
/// (bodyless WRITE_SEED = erase-all). The two read commands are case-2 APDUs
/// with an Le byte and are built by their callers.
fn transmit(w: &Wire, hdr: &[u8; 4], data: &[u8]) -> R<(Vec<u8>, u16)> {
    let mut apdu = hdr.to_vec();
    if !data.is_empty() {
        if data.len() <= 0xff {
            apdu.push(data.len() as u8);
        } else {
            apdu.extend_from_slice(&[0x00, (data.len() >> 8) as u8, (data.len() & 0xff) as u8]);
        }
        apdu.extend_from_slice(data);
    }
    transmit_raw(w, &apdu)
}
fn read_config_raw(w: &Wire) -> R<Vec<u8>> {
    // case-2: header + Le (number of bytes wanted; firmware fills the first 10)
    let (d, sw) = transmit_raw(w, &[READ_CONFIG[0], READ_CONFIG[1], READ_CONFIG[2], READ_CONFIG[3], 0x0a])?;
    if sw != 0x9000 { return Err(sw_err("READ_CONFIG", sw)); }
    Ok(d)
}
fn get_ecdh_pubkey_raw(w: &Wire) -> R<Vec<u8>> {
    let (d, sw) = transmit_raw(w, &[GET_ECDH_PUBKEY[0], GET_ECDH_PUBKEY[1], GET_ECDH_PUBKEY[2], GET_ECDH_PUBKEY[3], 0x00])?;
    if sw != 0x9000 { return Err(sw_err("GET_ECDH_PUBKEY", sw)); }
    Ok(d)
}
/// Send a fully formed APDU; follows 61xx with GET RESPONSE.
fn transmit_raw(w: &Wire, apdu: &[u8]) -> R<(Vec<u8>, u16)> {
    let mut cmd = apdu.to_vec();
    let mut data = Vec::new();
    loop {
        let (chunk, sw) = w.xmit_once(&cmd)?;
        data.extend_from_slice(&chunk);
        if sw >> 8 == 0x61 {
            cmd = vec![0x00, 0xc0, 0x00, 0x00, (sw & 0xff) as u8];
            continue;
        }
        if sw >> 8 == 0x6c {
            // wrong Le: resend the SAME apdu with the card-requested Le appended
            let le = (sw & 0xff) as u8;
            cmd = apdu.to_vec();
            if cmd.len() >= 5 && cmd[4] as usize == cmd.len() - 5 { cmd.push(le); } else if let Some(last) = cmd.last_mut() { *last = le; }
            data.clear();
            continue;
        }
        return Ok((data, sw));
    }
}
fn cmd(w: &Wire, what: &str, hdr: &[u8; 4], data: &[u8]) -> R<Vec<u8>> {
    let (d, sw) = transmit(w, hdr, data)?;
    if sw != 0x9000 {
        return Err(sw_err(what, sw));
    }
    Ok(d)
}
fn select(w: &Wire) -> R<()> {
    let mut a = vec![0x00, 0xa4, 0x04, 0x00, AID.len() as u8];
    a.extend_from_slice(&AID);
    let (_, sw) = transmit_raw(w, &a)?; // follows 61xx (T=0 readers)
    if sw != 0x9000 {
        return Err(if sw == 0x6a82 { "this key has no Token2 OTP applet".into() } else { sw_err("SELECT Token2 OTP", sw) });
    }
    Ok(())
}
fn now() -> R<u64> {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).map_err(|e| e.to_string())
}
/// SELECT with a one-shot recovery: on 6985 ("conditions not satisfied" — the
/// applet is mid-session from a prior op), cold-reset the card and re-select.
/// Because a reset invalidates any crypto session, also drop it so callers
/// rebuild it.
fn select_ready(s: &mut T2OtpSession) -> R<()> {
    let w = s.wire.as_ref().ok_or("No Token2 OTP device connected")?;
    // Over the HID tunnel the OTP applet is pre-selected and SELECT returns 6D00;
    // there is no card session to reset either. Skip the SELECT/reset dance.
    if matches!(w, Wire::Hid(_)) { return Ok(()); }
    match select(w) {
        Ok(()) => Ok(()),
        // 0x6985 (conditions not satisfied) or a reset of the shared card (another
        // applet/probe reset it) -> reconnect the card handle and re-select once.
        Err(e) if e.contains("0x6985") || e.to_lowercase().contains("reset") => {
            let _ = s.wire.as_mut().unwrap().reset();
            s.session = None;
            s.pin_verified = false;
            let w = s.wire.as_ref().unwrap();
            select(w)
        }
        Err(e) => Err(e),
    }
}
fn with_card<T>(state: &State<T2OtpState>, f: impl FnOnce(&Wire) -> R<T>) -> R<T> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_ready(&mut s)?;
    let w = s.wire.as_ref().unwrap();
    f(w)
}
fn with_session<T>(state: &State<T2OtpState>, f: impl FnOnce(&mut T2OtpSession) -> R<T>) -> R<T> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_ready(&mut s)?;
    f(&mut s)
}

// ---------------------------------------------------------- OTP-PIN crypto ---
fn hmac256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).unwrap();
    m.update(data);
    m.finalize().into_bytes().into()
}
fn sha256(d: &[u8]) -> [u8; 32] { Sha256::digest(d).into() }
fn random_iv() -> [u8; 16] { rand::random() }
fn aes_cbc_enc_nopad(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut buf = data.to_vec();
    let n = buf.len();
    Aes256CbcEnc::new(key.into(), iv.into()).encrypt_padded_mut::<NoPadding>(&mut buf, n).unwrap().to_vec()
}
fn aes_cbc_enc_pkcs7(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    Aes256CbcEnc::new(key.into(), iv.into()).encrypt_padded_vec_mut::<Pkcs7>(data)
}
fn aes_cbc_dec_nopad(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    if data.is_empty() || data.len() % 16 != 0 { return vec![]; }
    let mut buf = data.to_vec();
    Aes256CbcDec::new(key.into(), iv.into()).decrypt_padded_mut::<NoPadding>(&mut buf).map(|s| s.to_vec()).unwrap_or_default()
}
fn aes_cbc_dec_pkcs7(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> R<Vec<u8>> {
    if data.is_empty() || data.len() % 16 != 0 { return Err("bad ciphertext length".into()); }
    let mut buf = data.to_vec();
    Aes256CbcDec::new(key.into(), iv.into()).decrypt_padded_mut::<Pkcs7>(&mut buf).map(|s| s.to_vec()).map_err(|_| "decryption failed (bad padding)".into())
}
fn auth_tag(mac: &[u8; 32], data: &[u8]) -> [u8; 16] {
    let full = hmac256(mac, data);
    let mut t = [0u8; 16];
    t.copy_from_slice(&full[..16]);
    t
}
/// `IV || AES-CBC(enc, IV, clear) || HMAC(mac, ct)[:16]` — the format shared by
/// SET_OTP_PIN and PIN-protected WRITE_SEED.
fn sealed(keys: &SessionKeys, clear: &[u8]) -> Vec<u8> {
    let iv = random_iv();
    let ct = aes_cbc_enc_pkcs7(&keys.enc, &iv, clear);
    let tag = auth_tag(&keys.mac, &ct);
    let mut out = iv.to_vec();
    out.extend_from_slice(&ct);
    out.extend_from_slice(&tag);
    out
}
/// Client-side PIN policy (mirrors what the firmware rejects with 6A80).
fn validate_otp_pin(pin: &str) -> R<()> {
    let b = pin.as_bytes();
    if b.len() > 255 { return Err("PIN is too long".into()); }
    if b.iter().all(|c| c.is_ascii_digit()) {
        if b.len() < 6 { return Err("numeric PIN must be at least 6 digits".into()); }
        if b.iter().all(|&c| c == b[0]) { return Err("numeric PIN must not be all the same digit".into()); }
        if b.windows(2).all(|w| w[1] == w[0] + 1) || b.windows(2).all(|w| w[0] == w[1] + 1) { return Err("numeric PIN must not be a simple sequence".into()); }
        if b.iter().eq(b.iter().rev()) { return Err("numeric PIN must not be a palindrome".into()); }
        for d in b'0'..=b'9' { if b.iter().filter(|&&c| c == d).count() > 3 { return Err("numeric PIN repeats one digit too many times".into()); } }
    } else {
        if pin.chars().count() < 10 { return Err("alphanumeric PIN must be at least 10 characters".into()); }
        let classes = [pin.chars().any(|c| c.is_ascii_uppercase()), pin.chars().any(|c| c.is_ascii_lowercase()),
                       pin.chars().any(|c| c.is_ascii_digit()), pin.chars().any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace())]
            .iter().filter(|&&x| x).count();
        if classes < 2 { return Err("alphanumeric PIN needs at least two character classes".into()); }
    }
    Ok(())
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PinStatus {
    pub supported: bool,
    pub set: bool,
    pub retries_left: u8,
    pub max_retries: u8,
    pub pin_len: u8,
    pub fingerprint_enabled: bool, // FpEnable byte; set via t2otp_verify_pin(fp_enable)
    pub verified: bool,
}
/// READ_OTP_PIN_FLAG is a short-Lc command carrying `len` zero bytes; the
/// applet returns `len` bytes: AlgId Retry PinLen MaxRetry FpEnable PubVer(2)
/// PubCrc(2) [IV(16) EncRand(16)]. 6A86/6AF8 mean the firmware has no OTP-PIN.
/// Status words that mean the applet truly has no OTP-PIN command. `6AF8` is
/// NOT here: R3.4 answers a bodyless (priming) flag read with 6AF8 while fully
/// supporting the feature (per keyroost). 6982/6983/6985/6A81/63xx are real PIN
/// states, surfaced as errors, never "no feature".
const NO_PIN_FEATURE: [u16; 3] = [0x6d00, 0x6e00, 0x6a86];
fn read_pin_flag(w: &Wire, len: u8) -> R<Option<Vec<u8>>> {
    let mut a = READ_OTP_PIN_FLAG.to_vec();
    a.push(len);
    a.extend(std::iter::repeat(0u8).take(len as usize));
    let (d, sw) = transmit_raw(w, &a)?;
    match sw {
        0x9000 => Ok(Some(d)),
        _ if NO_PIN_FEATURE.contains(&sw) => Ok(None),
        0x6af8 | 0x6700 => Ok(Some(Vec::new())), // R3.4 priming answer: feature present, no data yet
        _ => Err(sw_err("READ_OTP_PIN_FLAG", sw)),
    }
}
fn pin_status_of(w: &Wire, verified: bool) -> R<PinStatus> {
    // A proper flag response gives the real state. Anything else at connect
    // time is treated as "supported, state unknown" unless the applet clearly
    // lacks the command (NO_PIN_FEATURE), so the UI does not wrongly disable it.
    let flag = match read_pin_flag(w, 0x09) {
        Ok(v) => v,
        Err(e) => {
            let no_feat = NO_PIN_FEATURE.iter().any(|w| e.contains(&format!("0x{w:04X}")) || e.contains(&format!("0x{w:04x}")));
            if no_feat { return Ok(PinStatus::default()); }
            Some(Vec::new())
        }
    };
    match flag {
        None => Ok(PinStatus::default()),
        Some(d) if d.len() < 4 => Ok(PinStatus { supported: true, set: false, verified, ..Default::default() }),
        Some(d) => {
            Ok(PinStatus { supported: true, set: d[2] != 0, retries_left: d[1], max_retries: d[3], pin_len: d[2],
                           fingerprint_enabled: d.get(4).copied().unwrap_or(0) != 0, verified })
        }
    }
}
fn ensure_session(s: &mut T2OtpSession) -> R<()> {
    if s.session.is_some() { return Ok(()); }
    let w = s.wire.as_ref().ok_or("No Token2 OTP device connected")?;
    // Prime the PIN state (result intentionally ignored, as in keyroost). The
    // agreement command below is the real feature gate: if the applet lacks
    // OTP-PIN it answers that with an unknown-instruction status word.
    let _ = read_pin_flag(w, 0x09);
    let eph = EphemeralSecret::random(&mut rand::thread_rng());
    let host_xy = EncodedPoint::from(eph.public_key()).as_bytes()[1..].to_vec();
    let (d, sw) = transmit(w, &READ_AGREEMENT_PUBKEY, &host_xy)?;
    if sw != 0x9000 {
        if NO_PIN_FEATURE.contains(&sw) || sw == 0x6af8 {
            return Err("this key's OTP applet has no PIN feature (needs R3.4 or newer firmware)".into());
        }
        return Err(sw_err("READ_AGREEMENT_PUBKEY", sw));
    }
    if d.len() < 64 { return Err("short agreement response".into()); }
    // devPub(64) || sig(132): the P-521 signature is not verified (as in the reference client)
    let mut point = vec![0x04u8];
    point.extend_from_slice(&d[..64]);
    let dev_pub = PublicKey::from_sec1_bytes(&point).map_err(|e| format!("device agreement key: {e}"))?;
    let shared = eph.diffie_hellman(&dev_pub);
    let pu1 = hmac256(&[0u8; 32], shared.raw_secret_bytes());
    let mut mac_info = b"TOTP HMAC key".to_vec(); mac_info.push(1);
    let mut enc_info = b"TOTP AES key".to_vec(); enc_info.push(1);
    s.session = Some(SessionKeys { enc: hmac256(&pu1, &enc_info), mac: hmac256(&pu1, &mac_info) });
    s.pin_verified = false;
    Ok(())
}
/// Fetch the verify challenge (Lc=0x29) and decrypt Rand under the session key.
fn challenge_rand(s: &T2OtpSession) -> R<[u8; 16]> {
    let w = s.wire.as_ref().unwrap();
    let keys = s.session.as_ref().ok_or("no session")?;
    let d = read_pin_flag(w, 0x29)?.ok_or("OTP PIN not supported")?;
    if d.len() < 41 { return Err("verify challenge missing".into()); }
    if d[2] == 0 { return Err("no OTP PIN is set".into()); }
    let iv: [u8; 16] = d[9..25].try_into().unwrap();
    let rand = aes_cbc_dec_nopad(&keys.enc, &iv, &d[25..41]);
    rand.try_into().map_err(|_| "bad challenge".into())
}
/// PIN verify; with `fp_enable = Some(x)` also sets the "OTP requires
/// fingerprint" flag (manual V1.3 §1.14): data = IV || PinHashEnc2 || EncConfig,
/// EncConfig = AES(SessionEnc, same IV, PKCS7(fp byte)).
fn verify_pin_inner(s: &mut T2OtpSession, pin: &str, fp_enable: Option<bool>) -> R<()> {
    ensure_session(s)?;
    let rand = challenge_rand(s)?;
    let keys = s.session.as_ref().unwrap();
    // inner = AES(SHA256(pin), SHA256(rand)[:16], rand); outer = AES(SessionEnc, IV, inner); data = IV || outer
    let pin_hash = sha256(pin.as_bytes());
    let iv2: [u8; 16] = sha256(&rand)[..16].try_into().unwrap();
    let inner = aes_cbc_enc_nopad(&pin_hash, &iv2, &rand);
    let iv = random_iv();
    let outer = aes_cbc_enc_nopad(&keys.enc, &iv, &inner);
    let mut data = iv.to_vec();
    data.extend_from_slice(&outer);
    if let Some(fp) = fp_enable {
        let mut cfg = vec![fp as u8];
        cfg.extend(std::iter::repeat(15u8).take(15)); // PKCS#7 pad of one byte to 16
        data.extend_from_slice(&aes_cbc_enc_nopad(&keys.enc, &iv, &cfg));
    }
    let w = s.wire.as_ref().unwrap();
    let (_, sw) = transmit(w, &VERIFY_OTP_PIN, &data)?;
    if sw != 0x9000 {
        return Err(match sw { 0x6982 => "wrong OTP PIN".into(), 0x6983 => "OTP PIN is blocked".into(), _ => sw_err("VERIFY_OTP_PIN", sw) });
    }
    s.pin_verified = true;
    Ok(())
}
/// After a verify, ENUM/READ_ONE pages come back as `IV || Enc || Auth`.
fn maybe_decrypt(s: &T2OtpSession, data: &[u8]) -> R<Vec<u8>> {
    if !s.pin_verified { return Ok(data.to_vec()); }
    let keys = s.session.as_ref().ok_or("no session")?;
    if data.len() < 48 { return Err("short encrypted page".into()); }
    let iv: [u8; 16] = data[..16].try_into().unwrap();
    let enc = &data[16..data.len() - 16];
    let tag = &data[data.len() - 16..];
    if auth_tag(&keys.mac, enc)[..] != *tag { return Err("page authentication failed".into()); }
    aes_cbc_dec_pkcs7(&keys.enc, &iv, enc)
}
/// Seed writes: ECDH blob on an unprotected key, session-sealed on a
/// PIN-protected one. With a PIN set the applet refuses GET_ECDH_PUBKEY
/// (6A81), so the window must be open first.
fn seal_write(s: &T2OtpSession, clear: &[u8], iv_unprotected: &[u8; 16]) -> R<Vec<u8>> {
    let w = s.wire.as_ref().unwrap();
    if s.pin_verified {
        return Ok(sealed(s.session.as_ref().ok_or("no session")?, clear));
    }
    if let Ok(Some(flag)) = read_pin_flag(w, 0x09) {
        if flag.len() > 2 && flag[2] != 0 {
            return Err("this key has an OTP PIN: unlock with the PIN or fingerprint first".into());
        }
    }
    encrypt_for_device(w, clear, iv_unprotected)
}

/// ECDH + AES-256-CBC wrap of `clear` with the given IV (spec §7).
fn encrypt_for_device(w: &Wire, clear: &[u8], iv: &[u8; 16]) -> R<Vec<u8>> {
    let pk = get_ecdh_pubkey_raw(w)?;
    if pk.len() != 64 {
        return Err(format!("device public key has unexpected length {}", pk.len()));
    }
    let mut point = vec![0x04u8];
    point.extend_from_slice(&pk);
    let dev_pub = PublicKey::from_sec1_bytes(&point).map_err(|e| format!("device public key: {e}"))?;
    let eph = EphemeralSecret::random(&mut rand::thread_rng());
    let host_pub = EncodedPoint::from(eph.public_key());
    let shared = eph.diffie_hellman(&dev_pub);
    let key = Sha256::digest(shared.raw_secret_bytes());
    let ct = Aes256CbcEnc::new(&key.into(), iv.into()).encrypt_padded_vec_mut::<Pkcs7>(clear);
    let mut blob = host_pub.as_bytes()[1..].to_vec(); // X || Y without 0x04
    blob.extend_from_slice(&ct);
    Ok(blob)
}

// ---------------------------------------------------------------- types ---
#[derive(Serialize, Clone, Debug, Default)]
pub struct T2Entry {
    pub kind: String,      // "totp" | "hotp"
    pub algorithm: String, // "SHA1" | "SHA256"
    pub timestep: u16,
    pub digits: u8,
    pub button: bool,
    pub app: String,
    pub account: String,
    pub code: String,      // present for TOTP without button in READ_ALL; always in READ_ONE
}

fn parse_entries(mut d: &[u8], full: bool) -> R<Vec<T2Entry>> {
    let mut out = Vec::new();
    while !d.is_empty() {
        if d.len() < 7 {
            return Err("truncated entry".into());
        }
        let ty = d[0];
        let alg = d[1];
        let step = u16::from_be_bytes([d[2], d[3]]);
        let digits = d[4];
        let btn = d[5] != 0;
        let al = d[6] as usize;
        d = &d[7..];
        if d.len() < al + 1 { return Err("truncated app name".into()); }
        let app = String::from_utf8_lossy(&d[..al]).into_owned();
        d = &d[al..];
        let nl = d[0] as usize;
        d = &d[1..];
        if d.len() < nl { return Err("truncated account name".into()); }
        let account = String::from_utf8_lossy(&d[..nl]).into_owned();
        d = &d[nl..];
        let is_totp = ty == 0x01;
        let mut code = String::new();
        if full || (is_totp && !btn) {
            if d.is_empty() { return Err("truncated code".into()); }
            let cl = d[0] as usize;
            d = &d[1..];
            if d.len() < cl { return Err("truncated code".into()); }
            code = String::from_utf8_lossy(&d[..cl]).into_owned();
            d = &d[cl..];
        }
        out.push(T2Entry {
            kind: if is_totp { "totp".into() } else { "hotp".into() },
            algorithm: if alg == 0xc2 { "SHA256".into() } else { "SHA1".into() },
            timestep: step, digits, button: btn, app, account, code,
        });
    }
    Ok(out)
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct T2Config {
    pub fido_disabled: bool,
    pub hotp_keystroke_disabled: bool,
    pub hotp_no_enter: bool,
    pub fido_pin_set: bool,
    pub hotp_supported: bool,
    pub fingerprint: bool,
    pub nfc: bool,
    pub hotp_long_press: bool,
    pub pin_locked: bool,
    pub button_hotp_configured: bool,
    pub totp_supported: bool,
    pub fido21: bool,
    pub fingerprint_enroll: bool,
    pub hotp_numpad: bool,
    pub ccid: bool,
    pub button_hotp_unsupported: bool,
    pub otp_requires_fingerprint: bool,
    pub mandatory_fingerprint_supported: bool,
    pub appearance: String,
    pub fido_version: String,
    pub raw: String,
}
fn parse_config(d: &[u8]) -> R<T2Config> {
    // older firmware (R3.2/R3.3) may return a shorter config block; pad so the
    // bits we don't have read as 0 rather than failing the whole connect.
    let mut buf = [0u8; 10];
    let n = d.len().min(10);
    buf[..n].copy_from_slice(&d[..n]);
    let (t, c, e) = (buf[0], buf[1], buf[9]);
    let b = |v: u8, bit: u8| v & (1 << (bit - 1)) != 0;
    Ok(T2Config {
        fido_disabled: b(t, 1), hotp_keystroke_disabled: b(t, 2),
        hotp_no_enter: b(c, 1), fido_pin_set: b(c, 2), hotp_supported: b(c, 3), fingerprint: b(c, 4),
        nfc: b(c, 5), hotp_long_press: b(c, 6), pin_locked: b(c, 7), button_hotp_configured: b(c, 8),
        totp_supported: b(e, 1), fido21: b(e, 2), fingerprint_enroll: b(e, 3), hotp_numpad: b(e, 4),
        ccid: b(e, 5), button_hotp_unsupported: b(e, 6), otp_requires_fingerprint: b(e, 7), mandatory_fingerprint_supported: b(e, 8),
        appearance: hex::encode(&d[2..6]),
        fido_version: format!("{}.{}.{}", d[6], d[7], d[8]),
        raw: hex::encode(d),
    })
}

#[derive(Serialize, Clone)]
pub struct T2OtpInfo {
    pub reader: String,
    pub config: T2Config,
    pub pin: PinStatus,
    pub serial: String,
    pub model: Option<crate::t2model::T2Model>,
}
/// Full device serial from the OTP applet (`80 33 00 00`, D1 10 … request).
fn read_serial(w: &Wire) -> String {
    let mut req = vec![0xd1u8, 0x10];
    req.extend_from_slice(&[0u8; 16]);
    match transmit(w, &[0x80, 0x33, 0x00, 0x00], &req) {
        Ok((d, 0x9000)) if d.len() >= 3 && d[0] == 0xd1 && d[1] as usize + 2 <= d.len() => String::from_utf8_lossy(&d[2..2 + d[1] as usize]).into_owned(),
        _ => String::new(),
    }
}

// ------------------------------------------------------------- commands ---
#[tauri::command]
pub fn t2otp_connect_hid(state: State<T2OtpState>, path: String) -> R<T2OtpInfo> {
    let hid = crate::t2otp_hid::OtpHid::open(&path).map_err(|e| format!("open HID: {e}"))?;
    let w = Wire::Hid(hid);
    // No SELECT over HID: the OTP applet is pre-selected on the CTAPHID tunnel.
    let cfg = read_config_raw(&w).and_then(|d| parse_config(&d)).unwrap_or_default();
    let pin = pin_status_of(&w, false).unwrap_or_default();
    let serial = read_serial(&w);
    let model = crate::t2model::from_serial(&serial);
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    *s = T2OtpSession::default();
    s.wire = Some(w);
    s.reader = path.clone();
    Ok(T2OtpInfo { reader: path, config: cfg, pin, serial, model })
}

#[tauri::command]
pub async fn t2otp_connect(state: State<'_, T2OtpState>, reader: String) -> R<T2OtpInfo> {
    let ctx = Context::establish(Scope::User).map_err(|e| format!("PC/SC: {e}"))?;
    let cname = std::ffi::CString::new(reader.as_str()).map_err(|e| e.to_string())?;
    let card = ctx.connect(&cname, ShareMode::Shared, Protocols::ANY).map_err(|e| format!("connect to reader: {e}"))?;
    let mut w = Wire::Pcsc(card);
    if let Err(e) = select(&w) {
        // 6985 "conditions not satisfied": the applet is still in a previous
        // session state (e.g. a pending fingerprint capture). A cold reset of
        // the card clears it; retry once.
        if e.contains("0x6985") {
            w.reset()?;
            select(&w)?;
        } else {
            return Err(e);
        }
    }
    let cfg = read_config_raw(&w).and_then(|d| parse_config(&d)).unwrap_or_default();
    let pin = pin_status_of(&w, false).unwrap_or_default();
    let serial = read_serial(&w);
    let model = crate::t2model::from_serial(&serial);
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    *s = T2OtpSession::default();
    s.wire = Some(w);
    s._ctx = Some(ctx);
    s.reader = reader.clone();
    Ok(T2OtpInfo { reader, config: cfg, pin, serial, model })
}

#[tauri::command]
pub fn t2otp_disconnect(state: State<T2OtpState>) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    // leave the applet in a clean state for the next session (and other apps)
    if let Some(Wire::Pcsc(card)) = s.wire.take() {
        let _ = card.disconnect(Disposition::ResetCard);
    }
    *s = T2OtpSession::default();
    Ok(())
}

#[tauri::command]
pub async fn t2otp_config(state: State<'_, T2OtpState>) -> R<T2Config> {
    with_card(&state, |c| parse_config(&read_config_raw(c)?))
}

/// READ_ALL with pagination; TOTP entries without button carry their current code.
#[tauri::command]
pub async fn t2otp_list(state: State<'_, T2OtpState>) -> R<Vec<T2Entry>> {
    let ts = now()?;
    with_session(&state, |s| {
        let c = s.wire.as_ref().unwrap();
        let mut req = vec![0x03u8];
        req.extend_from_slice(&ts.to_be_bytes());
        let (d, sw) = transmit(c, &ENUM_CODES, &req)?;
        if sw == 0x6a80 || sw == 0x6a83 {
            return Ok(vec![]); // empty token
        }
        if sw == 0x6982 {
            return Err("OTP PIN required: unlock first".into());
        }
        if sw != 0x9000 {
            return Err(sw_err("ENUM_CODES", sw));
        }
        let mut all = Vec::new();
        let mut page = maybe_decrypt(s, &d)?;
        loop {
            if page.is_empty() {
                break;
            }
            let more = page[0] & 0x80 != 0;
            page[0] &= 0x7f;
            all.extend(parse_entries(&page, false)?);
            if !more {
                break;
            }
            let raw = cmd(c, "ENUM_CODES_CONTINUE", &ENUM_CONTINUE, &ts.to_be_bytes())?;
            page = maybe_decrypt(s, &raw)?;
        }
        Ok(all)
    })
}

/// READ_ONE: always returns the code (HOTP advances; button entries wait for a touch).
#[tauri::command]
pub async fn t2otp_read(state: State<'_, T2OtpState>, app: String, account: String) -> R<T2Entry> {
    let ts = now()?;
    with_session(&state, |s| {
        let c = s.wire.as_ref().unwrap();
        let mut req = vec![0x01u8];
        req.extend_from_slice(&ts.to_be_bytes());
        req.push(app.len() as u8);
        req.extend_from_slice(app.as_bytes());
        req.push(account.len() as u8);
        req.extend_from_slice(account.as_bytes());
        let d = cmd(c, "READ_ONE", &ENUM_CODES, &req)?;
        let d = maybe_decrypt(s, &d)?;
        let mut v = parse_entries(&d, true)?;
        v.pop().ok_or("empty READ_ONE response".into())
    })
}

#[tauri::command]
pub async fn t2otp_write(
    state: State<'_, T2OtpState>,
    app: String,
    account: String,
    secret_base32: String,
    kind: String,
    algorithm: String,
    digits: u8,
    timestep: u16,
    button: bool,
) -> R<()> {
    let mut b32 = secret_base32.replace(' ', "").to_uppercase();
    while b32.len() % 8 != 0 { b32.push('='); }
    let seed = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &b32).ok_or("secret is not valid base32")?;
    if !app.is_ascii() || !account.is_ascii() { return Err("app and account names must be ASCII".into()); }
    if app.len() > 64 { return Err("app name must be at most 64 characters".into()); }
    if account.is_empty() || account.len() > 64 { return Err("account name must be 1–64 characters".into()); }
    if seed.is_empty() || seed.len() > 64 { return Err("secret must decode to 1–64 bytes".into()); }
    if !(4..=10).contains(&digits) { return Err("digits must be 4–10".into()); }
    if timestep == 0 { return Err("timestep must be at least 1".into()); }
    let mut clear = vec![
        if kind == "totp" { 0x01 } else { 0x00 },
        if algorithm == "SHA256" { 0xc2 } else { 0xc1 },
        (timestep >> 8) as u8, (timestep & 0xff) as u8,
        digits,
        button as u8,
        app.len() as u8,
    ];
    clear.extend_from_slice(app.as_bytes());
    clear.push(account.len() as u8);
    clear.extend_from_slice(account.as_bytes());
    clear.push(seed.len() as u8);
    clear.extend_from_slice(&seed);
    with_session(&state, |s| {
        let blob = seal_write(s, &clear, &IV_ENTRY)?;
        cmd(s.wire.as_ref().unwrap(), "WRITE_SEED", &WRITE_SEED, &blob).map(|_| ())
    })
}

#[tauri::command]
pub async fn t2otp_delete(state: State<'_, T2OtpState>, app: String, account: String) -> R<()> {
    let mut clear = vec![0u8, 0, 0, 0, 0, 0, app.len() as u8];
    clear.extend_from_slice(app.as_bytes());
    clear.push(account.len() as u8);
    clear.extend_from_slice(account.as_bytes());
    clear.push(0);
    with_session(&state, |s| {
        let blob = seal_write(s, &clear, &IV_ENTRY)?;
        cmd(s.wire.as_ref().unwrap(), "DELETE", &WRITE_SEED, &blob).map(|_| ())
    })
}

/// WRITE_SEED with no data erases every entry (the key asks for a touch).
#[tauri::command]
pub async fn t2otp_erase_all(state: State<'_, T2OtpState>) -> R<()> {
    with_card(&state, |c| cmd(c, "ERASE_ALL", &WRITE_SEED, &[]).map(|_| ()))
}

#[tauri::command]
pub async fn t2otp_enable_totp(state: State<'_, T2OtpState>, enabled: bool) -> R<()> {
    with_card(&state, |c| cmd(c, "ENABLE_TOTP", &ENABLE_TOTP, &[enabled as u8]).map(|_| ()))
}

/// Configure the keystroke HOTP slot (§6.6): seed + Enter / long-press / numpad options.
#[tauri::command]
pub async fn t2otp_set_button_hotp(
    state: State<'_, T2OtpState>,
    secret_base32: String,
    digits: u8,
    no_enter: bool,
    long_press: bool,
    numpad: bool,
) -> R<()> {
    let mut b32 = secret_base32.replace(' ', "").to_uppercase();
    while b32.len() % 8 != 0 { b32.push('='); }
    let seed = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &b32).ok_or("secret is not valid base32")?;
    if seed.is_empty() || seed.len() > 64 { return Err("secret must decode to 1–64 bytes".into()); }
    if digits != 6 && digits != 8 { return Err("button HOTP supports 6 or 8 digits".into()); }
    let mut clear = vec![digits, seed.len() as u8];
    clear.extend_from_slice(&seed);
    with_session(&state, |s| {
        let blob = seal_write(s, &clear, &IV_BUTTON)?;
        let c = s.wire.as_ref().unwrap();
        cmd(c, "WRITE_HOTP_SEED", &WRITE_HOTP_SEED, &blob)?;
        cmd(c, "CFG_HOTP_ENTER", &CFG_HOTP_ENTER, &[no_enter as u8])?;
        cmd(c, "CFG_HOTP_TOUCH", &CFG_HOTP_TOUCH, &[long_press as u8])?;
        cmd(c, "CFG_HOTP_KBD_TYPE", &CFG_HOTP_KBD, &[numpad as u8])?;
        Ok(())
    })
}

#[tauri::command]
pub async fn t2otp_delete_button_hotp(state: State<'_, T2OtpState>) -> R<()> {
    with_session(&state, |s| {
        let blob = seal_write(s, &[0, 0], &IV_BUTTON)?;
        cmd(s.wire.as_ref().unwrap(), "WRITE_HOTP_SEED", &WRITE_HOTP_SEED, &blob).map(|_| ())
    })
}

/// Keystroke options only (no seed change).
#[tauri::command]
pub async fn t2otp_set_button_options(state: State<'_, T2OtpState>, no_enter: bool, long_press: bool, numpad: bool) -> R<()> {
    with_card(&state, |c| {
        cmd(c, "CFG_HOTP_ENTER", &CFG_HOTP_ENTER, &[no_enter as u8])?;
        cmd(c, "CFG_HOTP_TOUCH", &CFG_HOTP_TOUCH, &[long_press as u8])?;
        cmd(c, "CFG_HOTP_KBD_TYPE", &CFG_HOTP_KBD, &[numpad as u8])?;
        Ok(())
    })
}

/// SET_DEVICE_TYPE (§6.8): bitmask of interfaces to DISABLE — bit1 FIDO,
/// bit2 keyboard (HOTP keystrokes), bit3 CCID. The GUI only lets the user
/// change the keyboard bit; FIDO and CCID are carried over from READ_CONFIG.
/// Refuses any mask that would leave no interface enabled, and refuses to
/// turn off CCID at all (this app talks to the key over CCID).
#[tauri::command]
pub async fn t2otp_set_keyboard_interface(state: State<'_, T2OtpState>, keyboard_enabled: bool) -> R<T2Config> {
    with_card(&state, |c| {
        let cfg = parse_config(&read_config_raw(c)?)?;
        let mut mask: u8 = 0;
        if cfg.fido_disabled { mask |= 0x01; }
        if !keyboard_enabled { mask |= 0x02; }
        // CCID bit intentionally never set
        if mask & 0x07 == 0x07 || (mask & 0x01 != 0 && mask & 0x02 != 0 && !cfg.ccid) {
            return Err("refusing: this would leave the key with no usable interface".into());
        }
        cmd(c, "SET_DEVICE_TYPE", &SET_DEVICE_TYPE, &[mask])?;
        // the key may re-enumerate; read back best-effort
        Ok(read_config_raw(c).and_then(|d| parse_config(&d)).unwrap_or(cfg))
    })
}

// --------------------------------------------------------- OTP-PIN commands ---
#[tauri::command]
pub async fn t2otp_pin_status(state: State<'_, T2OtpState>) -> R<PinStatus> {
    with_session(&state, |s| pin_status_of(s.wire.as_ref().unwrap(), s.pin_verified))
}

/// Verify the OTP PIN: opens the read/write window (~5 min on the device).
#[tauri::command]
pub async fn t2otp_verify_pin(state: State<'_, T2OtpState>, pin: String, fp_enable: Option<bool>) -> R<PinStatus> {
    with_session(&state, |s| {
        verify_pin_inner(s, &pin, fp_enable)?;
        pin_status_of(s.wire.as_ref().unwrap(), true)
    })
}

/// Unlock with a fingerprint instead of the PIN (manual V1.3 §1.20):
/// `80 C5 05 06 01 01` answers 9100 while capturing; poll `80 11 00 00 00`
/// until 9000. Only works when the fingerprint flag is enabled.
#[tauri::command]
pub async fn t2otp_verify_fingerprint(state: State<'_, T2OtpState>) -> R<PinStatus> {
    with_session(&state, |s| {
        ensure_session(s)?;
        let w = s.wire.as_ref().unwrap();
        let mut a = VERIFY_OTP_PIN.to_vec();
        a.extend_from_slice(&[0x01, 0x01]);
        let (_, mut sw) = transmit_raw(w, &a)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while sw == 0x9100 {
            if std::time::Instant::now() > deadline { return Err("timed out waiting for the fingerprint".into()); }
            std::thread::sleep(std::time::Duration::from_millis(250));
            let (_, sw2) = transmit_raw(w, &[0x80, 0x11, 0x00, 0x00, 0x00])?;
            sw = sw2;
        }
        if sw != 0x9000 {
            return Err(match sw { 0x6982 => "fingerprint not recognised".into(), 0x6a81 => "fingerprint unlock is not enabled for OTP".into(), _ => sw_err("FINGERPRINT_VERIFY", sw) });
        }
        s.pin_verified = true;
        pin_status_of(s.wire.as_ref().unwrap(), true)
    })
}

/// Set a PIN on an unprotected key.
#[tauri::command]
pub async fn t2otp_set_pin(state: State<'_, T2OtpState>, pin: String) -> R<PinStatus> {
    validate_otp_pin(&pin)?;
    with_session(&state, |s| {
        ensure_session(s)?;
        let keys = s.session.as_ref().unwrap();
        let mut block = vec![PIN_ALG_AES256, PIN_MAX_RETRY, pin.len() as u8];
        block.extend_from_slice(pin.as_bytes());
        let data = sealed(keys, &block);
        let w = s.wire.as_ref().unwrap();
        let (_, sw) = transmit(w, &SET_OTP_PIN, &data)?;
        if sw != 0x9000 {
            return Err(match sw { 0x6a81 => "a PIN is already set — use change".into(), _ => sw_err("SET_OTP_PIN", sw) });
        }
        // the reference verifies right away so the window is open
        verify_pin_inner(s, &pin, None).ok();
        pin_status_of(s.wire.as_ref().unwrap(), s.pin_verified)
    })
}

/// Change the PIN (new_pin non-empty) or remove it (new_pin empty).
#[tauri::command]
pub async fn t2otp_change_pin(state: State<'_, T2OtpState>, current: String, new_pin: String) -> R<PinStatus> {
    if !new_pin.is_empty() { validate_otp_pin(&new_pin)?; }
    with_session(&state, |s| {
        ensure_session(s)?;
        let _rand = challenge_rand(s)?; // binds the change to a fresh challenge, as the reference does
        let keys = s.session.as_ref().unwrap();
        let mut body = vec![PIN_ALG_AES256, PIN_MAX_RETRY, new_pin.len() as u8];
        body.extend_from_slice(new_pin.as_bytes());
        let n = 16 - body.len() % 16;
        body.extend(std::iter::repeat(n as u8).take(n));
        let iv = random_iv();
        let new_enc = aes_cbc_enc_nopad(&keys.enc, &iv, &body);
        let old_hash: [u8; 16] = sha256(current.as_bytes())[..16].try_into().unwrap();
        let old_enc = aes_cbc_enc_nopad(&keys.enc, &iv, &old_hash);
        let mut mac_in = new_enc.clone();
        mac_in.extend_from_slice(&old_enc);
        let tag = auth_tag(&keys.mac, &mac_in);
        let mut data = iv.to_vec();
        data.extend_from_slice(&new_enc);
        data.extend_from_slice(&tag);
        data.extend_from_slice(&old_enc);
        let w = s.wire.as_ref().unwrap();
        let (_, sw) = transmit(w, &CHANGE_OTP_PIN, &data)?;
        if sw != 0x9000 {
            return Err(match sw { 0x6982 => "current OTP PIN is wrong".into(), 0x6983 => "OTP PIN is blocked".into(), _ => sw_err("CHANGE_OTP_PIN", sw) });
        }
        s.pin_verified = false;
        pin_status_of(s.wire.as_ref().unwrap(), false)
    })
}

/// Close the read/write window now (VERIFY header with a single 0x00 body).
#[tauri::command]
pub async fn t2otp_lock(state: State<'_, T2OtpState>) -> R<PinStatus> {
    with_session(&state, |s| {
        let w = s.wire.as_ref().unwrap();
        let mut a = VERIFY_OTP_PIN.to_vec();
        a.extend_from_slice(&[0x01, 0x00]);
        let (_, sw) = transmit_raw(w, &a)?;
        if sw != 0x9000 { return Err(sw_err("LOCK", sw)); }
        s.pin_verified = false;
        pin_status_of(s.wire.as_ref().unwrap(), false)
    })
}

// ------------------------------------------------ CTAP2 tunnel (Token2) -----
// The Token2 tool configures FIDO through the OTP applet: `80 C5 03 00` wraps a
// CTAP2 command (cmd byte + CBOR). `80 C5 02 0A` wraps authenticatorConfig
// setMinPINLength but applies it to the *alphanumeric* minimum. Observed in
// USBPcap captures of the Token2 Windows tool (Sept 2026).
const CTAP_TUNNEL: [u8; 4] = [0x80, 0xc5, 0x03, 0x00];
const CTAP_ALPHA_MINPIN: [u8; 4] = [0x80, 0xc5, 0x02, 0x0a];

fn ctap_err(code: u8) -> String {
    match code {
        0x31 => "wrong PIN".into(),
        0x32 => "PIN is blocked".into(),
        0x33 => "PIN authentication invalid".into(),
        0x34 => "PIN authentication blocked — reinsert the key".into(),
        0x35 => "no PIN is set".into(),
        0x36 => "PIN required".into(),
        0x37 => "PIN policy violation (minimum length not met, or forced change pending)".into(),
        0x2b => "unsupported option".into(),
        0x01 => "invalid command".into(),
        0x02 => "invalid parameter".into(),
        c => format!("CTAP error 0x{c:02x}"),
    }
}
/// Send a tunnelled CTAP2 command; returns the CBOR payload after the status byte.
fn ctap_tunnel(w: &Wire, hdr: &[u8; 4], cmd: u8, cbor: &[u8]) -> R<Vec<u8>> {
    let mut body = vec![cmd];
    body.extend_from_slice(cbor);
    let (d, sw) = transmit(w, hdr, &body)?;
    if sw != 0x9000 { return Err(sw_err("CTAP tunnel", sw)); }
    if d.is_empty() { return Err("empty CTAP response".into()); }
    if d[0] != 0x00 { return Err(ctap_err(d[0])); }
    Ok(d[1..].to_vec())
}
/// Find a 32-byte byte string that follows the given CBOR key bytes.
fn cbor_bstr32_after(d: &[u8], key: &[u8]) -> Option<[u8; 32]> {
    let mut pat = key.to_vec();
    pat.extend_from_slice(&[0x58, 0x20]);
    d.windows(pat.len()).position(|w| w == pat.as_slice()).and_then(|i| d.get(i + pat.len()..i + pat.len() + 32)).and_then(|s| s.try_into().ok())
}

/// Set the *alphanumeric* minimum PIN length via the Token2 tunnel
/// (PIN protocol 1: ECDH P-256, SHA-256 shared secret, AES-256-CBC zero IV,
/// HMAC-SHA256 pinUvAuthParam truncated to 16 bytes).
#[tauri::command]
pub async fn t2otp_set_alpha_min_pin_len(state: State<'_, T2OtpState>, len: u8, pin: String) -> R<()> {
    if !(4..=63).contains(&len) { return Err("length must be 4–63".into()); }
    with_card(&state, |w| {
        // 1. getKeyAgreement
        let ka = ctap_tunnel(w, &CTAP_TUNNEL, 0x06, &[0xa2, 0x01, 0x01, 0x02, 0x02])?;
        let x = cbor_bstr32_after(&ka, &[0x21]).ok_or("no key-agreement x")?;
        let y = cbor_bstr32_after(&ka, &[0x22]).ok_or("no key-agreement y")?;
        let mut dev_pt = vec![0x04u8];
        dev_pt.extend_from_slice(&x);
        dev_pt.extend_from_slice(&y);
        let dev_pub = PublicKey::from_sec1_bytes(&dev_pt).map_err(|e| format!("authenticator key: {e}"))?;
        let eph = EphemeralSecret::random(&mut rand::thread_rng());
        let host_pt = EncodedPoint::from(eph.public_key());
        let (hx, hy) = (host_pt.x().unwrap(), host_pt.y().unwrap());
        let shared: [u8; 32] = sha256(eph.diffie_hellman(&dev_pub).raw_secret_bytes()); // PIN protocol 1
        // 2. getPinUvAuthTokenUsingPinWithPermissions (0x09), permissions 0x20 = authenticatorConfig
        let pin_hash: [u8; 16] = sha256(pin.as_bytes())[..16].try_into().unwrap();
        let pin_hash_enc = aes_cbc_enc_nopad(&shared, &[0u8; 16], &pin_hash);
        let mut c = vec![0xa5, 0x01, 0x01, 0x02, 0x09, 0x03,
                         0xa5, 0x01, 0x02, 0x03, 0x38, 0x18, 0x20, 0x01, 0x21, 0x58, 0x20];
        c.extend_from_slice(hx);
        c.extend_from_slice(&[0x22, 0x58, 0x20]);
        c.extend_from_slice(hy);
        c.extend_from_slice(&[0x06, 0x50]);
        c.extend_from_slice(&pin_hash_enc);
        c.extend_from_slice(&[0x09, 0x18, 0x20]);
        let tok = ctap_tunnel(w, &CTAP_TUNNEL, 0x06, &c)?;
        let tok_enc = cbor_bstr32_after(&tok, &[0x02]).ok_or("no pinUvAuthToken in response")?;
        let token = aes_cbc_dec_nopad(&shared, &[0u8; 16], &tok_enc);
        if token.len() != 32 { return Err("could not decrypt pinUvAuthToken".into()); }
        // 3. authenticatorConfig setMinPINLength through the alphanumeric header
        let params = vec![0xa1u8, 0x01, len_cbor(len)].into_iter().chain(len_cbor_tail(len)).collect::<Vec<u8>>();
        let mut msg = vec![0xffu8; 32];
        msg.push(0x0d);
        msg.push(0x03);
        msg.extend_from_slice(&params);
        let auth: [u8; 16] = hmac256(&token, &msg)[..16].try_into().unwrap();
        let mut cfg = vec![0xa4, 0x01, 0x03, 0x02];
        cfg.extend_from_slice(&params);
        cfg.extend_from_slice(&[0x03, 0x01, 0x04, 0x50]);
        cfg.extend_from_slice(&auth);
        ctap_tunnel(w, &CTAP_ALPHA_MINPIN, 0x0d, &cfg).map(|_| ())
    })
}
// tiny CBOR unsigned-int helpers (0..63 short form, else 0x18 nn)
fn len_cbor(n: u8) -> u8 { if n < 24 { n } else { 0x18 } }
fn len_cbor_tail(n: u8) -> Vec<u8> { if n < 24 { vec![] } else { vec![n] } }

// --------------------------------------------------- applet activation --------
/// Applet-mode byte as read by the Token2 tool (`80 33 FB 01 01 00`). The bit
/// layout is not published; only "08 = everything on" has been observed, so
/// the value is exposed raw and the UI treats non-08 as "some applet off".
#[tauri::command]
pub async fn t2otp_applet_mode(state: State<'_, T2OtpState>) -> R<u8> {
    with_card(&state, |c| {
        let (d, sw) = transmit_raw(c, &[0x80, 0x33, 0xfb, 0x01, 0x01, 0x00])?;
        if sw != 0x9000 { return Err(sw_err("READ_APPLET_MODE", sw)); }
        d.first().copied().ok_or("empty applet-mode response".into())
    })
}
/// Device-info block (`80 33 00 00 02 BB 00`, 27 bytes) — returned raw for now.
#[tauri::command]
pub async fn t2otp_device_info_raw(state: State<'_, T2OtpState>) -> R<String> {
    with_card(&state, |c| {
        let (d, sw) = transmit_raw(c, &[0x80, 0x33, 0x00, 0x00, 0x02, 0xbb, 0x00])?;
        if sw != 0x9000 { return Err(sw_err("READ_DEVICE_INFO", sw)); }
        Ok(hex::encode(d))
    })
}
/// Applet enable/disable (R3.4, from the Token2 applet script). The flag byte
/// read via `80 33 FB 01`:
///   bit0 OpenPGP/NFC, bit1 PIV/NFC, bit4 OpenPGP/USB(7816), bit5 PIV/USB(7816)
///   (1 = disabled, 0 = enabled; bits 2,3,6,7 preserved).
/// Write: `80 33 FF FF 10 <const>` → 16-byte Rand; then
///   VerData = AES-192-ECB(EncKey, Rand)
///   EncData = AES-192-CBC(EncKey, IV=Rand, NewFlag ‖ Rand[0..15])
///   `80 33 FB 00 20 VerData‖EncData`.
const APPLET_ENC_KEY: [u8; 24] = [
    0xc9,0x47,0xa2,0x76,0x29,0xce,0x37,0xa7,0x1f,0x96,0x08,0x4c,
    0x0b,0x1a,0x84,0x62,0x03,0x0d,0x98,0xa4,0x35,0xa2,0xda,0xa9,
];
const APPLET_CHALLENGE_CONST: [u8; 16] = [
    0xe9,0x31,0xb6,0x88,0xbd,0x97,0xc0,0x8c,0x0e,0xcc,0x92,0xbd,0x42,0x2d,0x2e,0x60,
];

#[derive(Serialize, Clone, Default)]
pub struct AppletFlags {
    pub raw: u8,
    pub openpgp_nfc: bool, // true = enabled
    pub piv_nfc: bool,
    pub openpgp_usb: bool,
    pub piv_usb: bool,
}
fn decode_flags(b: u8) -> AppletFlags {
    AppletFlags {
        raw: b,
        openpgp_nfc: b & 0x01 == 0,
        piv_nfc: b & 0x02 == 0,
        openpgp_usb: b & 0x10 == 0,
        piv_usb: b & 0x20 == 0,
    }
}

#[tauri::command]
pub async fn t2otp_applet_flags(state: State<'_, T2OtpState>) -> R<AppletFlags> {
    with_card(&state, |c| {
        let (d, sw) = transmit_raw(c, &[0x80, 0x33, 0xfb, 0x01, 0x01, 0x00])?;
        if sw != 0x9000 { return Err(sw_err("READ_APPLET_FLAGS", sw)); }
        Ok(decode_flags(*d.first().ok_or("empty flag response")?))
    })
}


fn aes192_ecb_block(key: &[u8; 24], block16: &[u8]) -> Vec<u8> {
    use aes::cipher::{BlockEncrypt, KeyInit};
    let cipher = aes::Aes192::new(key.into());
    let mut b = [0u8; 16];
    b.copy_from_slice(&block16[..16]);
    let mut ga = aes::cipher::generic_array::GenericArray::clone_from_slice(&b);
    cipher.encrypt_block(&mut ga);
    ga.to_vec()
}
fn aes192_cbc_nopad(key: &[u8; 24], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    // data must be a multiple of 16 (here 16 bytes: NewFlag + 15 rand bytes)
    let mut buf = data.to_vec();
    let n = buf.len();
    use aes::cipher::KeyIvInit;
    cbc::Encryptor::<aes::Aes192>::new(key.into(), iv.into())
        .encrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut buf, n).unwrap().to_vec()
}

/// Set the applet enable state. `usb_pgp/usb_piv/nfc_pgp/nfc_piv` = desired
/// enabled state; other flag bits are preserved.
#[tauri::command]
pub async fn t2otp_set_applet_flags(
    state: State<'_, T2OtpState>,
    usb_pgp: bool, usb_piv: bool, nfc_pgp: bool, nfc_piv: bool,
) -> R<AppletFlags> {
    with_card(&state, |c| {
        // read current flag
        let (d, sw) = transmit_raw(c, &[0x80, 0x33, 0xfb, 0x01, 0x01, 0x00])?;
        if sw != 0x9000 { return Err(sw_err("READ_APPLET_FLAGS", sw)); }
        let cur = *d.first().ok_or("empty flag response")?;
        // build new flag: set bit to 1 to DISABLE
        let mut nf = cur;
        let setbit = |v: &mut u8, bit: u8, enabled: bool| { if enabled { *v &= !bit; } else { *v |= bit; } };
        setbit(&mut nf, 0x01, nfc_pgp);
        setbit(&mut nf, 0x02, nfc_piv);
        setbit(&mut nf, 0x10, usb_pgp);
        setbit(&mut nf, 0x20, usb_piv);
        if nf == cur { return Ok(decode_flags(cur)); }
        // challenge
        let mut ch = vec![0x80u8, 0x33, 0xff, 0xff, 0x10];
        ch.extend_from_slice(&APPLET_CHALLENGE_CONST);
        let (rand, sw) = transmit_raw(c, &ch)?;
        if sw != 0x9000 || rand.len() < 16 { return Err(sw_err("APPLET_CHALLENGE", sw)); }
        let iv: [u8; 16] = rand[..16].try_into().unwrap();
        // VerData = ECB(EncKey, Rand)
        let ver = aes192_ecb_block(&APPLET_ENC_KEY, &rand);
        // EncData = CBC(EncKey, IV=Rand, NewFlag ‖ Rand[0..15])
        let mut plain = vec![nf];
        plain.extend_from_slice(&rand[..15]);
        let enc = aes192_cbc_nopad(&APPLET_ENC_KEY, &iv, &plain);
        // 80 33 FB 00 20 VerData‖EncData  (0x20 = 32 bytes)
        let mut body = ver;
        body.extend_from_slice(&enc);
        let mut apdu = vec![0x80u8, 0x33, 0xfb, 0x00, body.len() as u8];
        apdu.extend_from_slice(&body);
        let (_, sw) = transmit_raw(c, &apdu)?;
        if sw != 0x9000 { return Err(sw_err("WRITE_APPLET_FLAGS", sw)); }
        // read back
        let (d2, sw2) = transmit_raw(c, &[0x80, 0x33, 0xfb, 0x01, 0x01, 0x00])?;
        Ok(decode_flags(if sw2 == 0x9000 { *d2.first().unwrap_or(&nf) } else { nf }))
    })
}

/// Full OTP applet reset (protocol §6.7): clears all accounts, the OTP PIN and
/// button-HOTP. `80 C5 05 03` with the confirmation bytes; needs a button press.
#[tauri::command]
pub async fn t2otp_applet_reset(state: State<'_, T2OtpState>) -> R<()> {
    with_card(&state, |c| {
        // erase entries first (bodyless WRITE_SEED), then clear the PIN state
        let _ = cmd(c, "ERASE_ALL", &WRITE_SEED, &[]);
        let (_, sw) = transmit_raw(c, &[0x80, 0xc5, 0x05, 0x03, 0x02, 0xde, 0xad])?;
        if sw != 0x9000 && sw != 0x6a80 && sw != 0x6a83 {
            return Err(sw_err("OTP_APPLET_RESET", sw));
        }
        Ok(())
    })
}

// ------------------------------------------------------------ QR scan --------
/// Decode QR codes from a PNG/JPEG image (a screenshot the user pasted or a
/// file they picked) and return the otpauth:// URIs found. Pure Rust (rqrr);
/// the image is size-capped before allocation.
#[tauri::command]
pub async fn qr_decode_image(png: Vec<u8>) -> R<Vec<String>> {
    qr_texts(&png)
}

/// Grab the screen(s) and decode any QR codes on them (Windows/macOS/Linux).
#[tauri::command]
pub async fn qr_scan_screen() -> R<Vec<String>> {
    let images = capture_screens()?;
    let mut out = Vec::new();
    for img in images {
        if let Ok(mut t) = qr_texts_rgba(&img.rgba, img.width, img.height) {
            out.append(&mut t);
        }
    }
    if out.is_empty() {
        return Err("No QR code found on screen. Make the authenticator QR fully visible and try again.".into());
    }
    Ok(out)
}

const MAX_QR_PIXELS: u64 = 60_000_000;

fn qr_texts(bytes: &[u8]) -> R<Vec<String>> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("could not decode image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if (w as u64) * (h as u64) > MAX_QR_PIXELS {
        return Err("image is too large to scan".into());
    }
    let luma = img.to_luma8();
    qr_from_luma(luma.as_raw(), w, h)
}
fn qr_texts_rgba(rgba: &[u8], w: u32, h: u32) -> R<Vec<String>> {
    if (w as u64) * (h as u64) > MAX_QR_PIXELS || rgba.len() < (w as usize) * (h as usize) * 4 {
        return Err("frame too large".into());
    }
    let mut luma = vec![0u8; (w as usize) * (h as usize)];
    for (i, px) in rgba.chunks_exact(4).enumerate() {
        // Rec.601 luma
        luma[i] = ((px[0] as u32 * 77 + px[1] as u32 * 150 + px[2] as u32 * 29) >> 8) as u8;
    }
    qr_from_luma(&luma, w, h)
}
fn qr_from_luma(luma: &[u8], w: u32, h: u32) -> R<Vec<String>> {
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w as usize, h as usize, |x, y| luma[y * w as usize + x]);
    let grids = img.detect_grids();
    let mut out = Vec::new();
    for g in grids {
        if let Ok((_meta, content)) = g.decode() {
            if content.starts_with("otpauth://") || content.starts_with("otpauth-migration://") {
                out.push(content);
            }
        }
    }
    Ok(out)
}

// ---- screen capture backends ----
struct Capture { rgba: Vec<u8>, width: u32, height: u32 }

#[cfg(windows)]
fn capture_screens() -> R<Vec<Capture>> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if w <= 0 || h <= 0 { return Err("could not read screen size".into()); }
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp);
        let _ = BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        GetDIBits(mem, bmp, 0, h as u32, Some(buf.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        // BGRA -> RGBA
        for px in buf.chunks_exact_mut(4) { px.swap(0, 2); px[3] = 255; }
        SelectObject(mem, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        Ok(vec![Capture { rgba: buf, width: w as u32, height: h as u32 }])
    }
}

#[cfg(target_os = "linux")]
fn capture_screens() -> R<Vec<Capture>> {
    Err("screen scan isn't available on this build; use 'Scan from image file' instead".into())
}

#[cfg(target_os = "macos")]
fn capture_screens() -> R<Vec<Capture>> {
    use std::process::Command;
    let tmp = std::env::temp_dir().join("t2_qr_scan.png");
    let out = Command::new("/usr/sbin/screencapture").arg("-x").arg(&tmp).output().map_err(|e| e.to_string())?;
    if !out.status.success() { return Err("screencapture failed".into()); }
    let bytes = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let (w, h) = (img.width(), img.height());
    Ok(vec![Capture { rgba: img.to_rgba8().into_raw(), width: w, height: h }])
}

/// Pick a PNG/JPEG file and decode QR codes from it.
#[tauri::command]
pub async fn qr_scan_file() -> R<Vec<String>> {
    let Some(path) = rfd::FileDialog::new().add_filter("Image", &["png", "jpg", "jpeg"]).pick_file() else {
        return Ok(vec![]);
    };
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let v = qr_texts(&bytes)?;
    if v.is_empty() { return Err("No otpauth QR code found in that image.".into()); }
    Ok(v)
}
