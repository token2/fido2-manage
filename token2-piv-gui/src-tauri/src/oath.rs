//! OATH applet (TOTP/HOTP) over PC/SC. Protocol as implemented in Token2's
//! Libre Key Companion: AID A0 00 00 05 27 21 01, INS PUT/DELETE/SET_CODE/
//! RESET/LIST/CALCULATE/VALIDATE/CALCULATE_ALL, optional password with
//! PBKDF2-HMAC-SHA1(password, salt = device id, 1000 iterations, 16 bytes) and
//! HMAC challenge-response (mutual) authentication.

use hmac::{Hmac, Mac};
use pcsc::{Card, Context, Protocols, Scope, ShareMode};
use serde::Serialize;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::sync::Mutex;
use tauri::State;

type R<T> = Result<T, String>;

const AID: [u8; 7] = [0xa0, 0x00, 0x00, 0x05, 0x27, 0x21, 0x01];
const INS_PUT: u8 = 0x01;
const INS_DELETE: u8 = 0x02;
const INS_SET_CODE: u8 = 0x03;
const INS_RESET: u8 = 0x04;
const INS_LIST: u8 = 0xa1;
const INS_CALCULATE: u8 = 0xa2;
const INS_VALIDATE: u8 = 0xa3;
const INS_CALCULATE_ALL: u8 = 0xa4;
const TAG_NAME: u8 = 0x71;
const TAG_NAME_LIST: u8 = 0x72;
const TAG_KEY: u8 = 0x73;
const TAG_CHALLENGE: u8 = 0x74;
const TAG_RESPONSE_FULL: u8 = 0x75;
const TAG_RESPONSE_TRUNC: u8 = 0x76;
const TAG_NO_RESPONSE: u8 = 0x77;
const TAG_PROPERTY: u8 = 0x78;
const TAG_VERSION: u8 = 0x79;
const TAG_TOUCH: u8 = 0x7c;
const TAG_ALGORITHM: u8 = 0x7b;
const TYPE_HOTP: u8 = 0x10;
const TYPE_TOTP: u8 = 0x20;
const ALG_SHA1: u8 = 0x01;
const ALG_SHA256: u8 = 0x02;
const ALG_SHA512: u8 = 0x03;
const PROP_TOUCH: u8 = 0x02;
const MIN_KEY: usize = 14;

fn tlv(tag: u8, v: &[u8]) -> Vec<u8> {
    let mut out = vec![tag, v.len() as u8];
    out.extend_from_slice(v);
    out
}
fn parse_tlvs(d: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 <= d.len() {
        let (t, l) = (d[i], d[i + 1] as usize);
        i += 2;
        if i + l > d.len() {
            break;
        }
        out.push((t, d[i..i + l].to_vec()));
        i += l;
    }
    out
}
fn hmac_alg(alg: u8, key: &[u8], msg: &[u8]) -> Vec<u8> {
    match alg {
        ALG_SHA256 => { let mut m = Hmac::<Sha256>::new_from_slice(key).unwrap(); m.update(msg); m.finalize().into_bytes().to_vec() }
        ALG_SHA512 => { let mut m = Hmac::<Sha512>::new_from_slice(key).unwrap(); m.update(msg); m.finalize().into_bytes().to_vec() }
        _ => { let mut m = Hmac::<Sha1>::new_from_slice(key).unwrap(); m.update(msg); m.finalize().into_bytes().to_vec() }
    }
}
pub fn derive_access_key(password: &str, device_id: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<Sha1>(password.as_bytes(), device_id, 1000, &mut out);
    out
}

pub struct OathSession {
    card: Option<Card>,
    hid: Option<crate::t2otp_hid::OtpHid>,
    reader: String,
    device_id: Vec<u8>,
    version: String,
    challenge: Vec<u8>,
    algorithm: u8,
    password_set: bool,
    access_key: Option<[u8; 16]>,
    _ctx: Option<Context>,
}
impl Default for OathSession {
    fn default() -> Self {
        Self { card: None, hid: None, reader: String::new(), device_id: vec![], version: String::new(), challenge: vec![], algorithm: ALG_SHA1, password_set: false, access_key: None, _ctx: None }
    }
}
pub struct OathState(pub Mutex<OathSession>);

fn raw_xmit(s: &OathSession, apdu: &[u8]) -> R<(Vec<u8>, u16)> {
    if let Some(hid) = s.hid.as_ref() { return hid.transmit(apdu); }
    let card = s.card.as_ref().ok_or("No OATH device connected")?;
    let mut buf = vec![0u8; 4096];
    let r = card.transmit(apdu, &mut buf).map_err(|e| format!("PC/SC transmit: {e}"))?;
    if r.len() < 2 { return Err("short APDU response".into()); }
    let sw = ((r[r.len()-2] as u16) << 8) | r[r.len()-1] as u16;
    Ok((r[..r.len()-2].to_vec(), sw))
}
fn transmit(s: &OathSession, apdu: &[u8]) -> R<(Vec<u8>, u16)> {
    let mut cmd = apdu.to_vec();
    let mut data = Vec::new();
    loop {
        let (chunk, sw) = raw_xmit(s, &cmd)?;
        data.extend_from_slice(&chunk);
        if sw >> 8 == 0x61 {
            let (chunk2, sw2) = raw_xmit(s, &[0x00, 0xa5, 0x00, 0x00, (sw & 0xff) as u8])
                .or_else(|_| raw_xmit(s, &[0x00, 0xc0, 0x00, 0x00, (sw & 0xff) as u8]))?;
            data.extend_from_slice(&chunk2);
            if sw2 >> 8 == 0x61 { cmd = vec![0x00, 0xc0, 0x00, 0x00, (sw2 & 0xff) as u8]; continue; }
            return Ok((data, sw2));
        }
        return Ok((data, sw));
    }
}
fn apdu(ins: u8, p1: u8, p2: u8, body: &[u8]) -> Vec<u8> {
    let mut a = vec![0x00, ins, p1, p2];
    if !body.is_empty() {
        a.push(body.len() as u8);
        a.extend_from_slice(body);
    }
    a.push(0x00);
    a
}
fn sw_err(what: &str, sw: u16) -> String {
    match sw {
        0x6982 => "the OATH applet is password-protected — unlock it first".into(),
        0x6a80 | 0x6a84 => format!("{what}: rejected by the applet (0x{sw:04X}) — name too long, invalid secret, or storage full"),
        0x6984 => format!("{what}: no such credential"),
        0x6985 => format!("{what}: touch required — touch the key and try again"),
        _ => format!("{what}: status 0x{sw:04X}"),
    }
}

/// SELECT the applet; refreshes version/device id/challenge and, if a
/// password is set and known, re-validates (the challenge is per-selection).
fn select_and_auth(s: &mut OathSession) -> R<()> {
    let mut a = vec![0x00, 0xa4, 0x04, 0x00, AID.len() as u8];
    a.extend_from_slice(&AID);
    let (d, sw) = transmit(&*s, &a)?;
    if sw != 0x9000 {
        return Err(if sw == 0x6a82 { "this key has no OATH applet".into() } else { sw_err("SELECT OATH", sw) });
    }
    s.challenge.clear();
    s.algorithm = ALG_SHA1;
    for (t, v) in parse_tlvs(&d) {
        match t {
            TAG_VERSION => s.version = v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."),
            TAG_NAME => s.device_id = v,
            TAG_CHALLENGE => s.challenge = v,
            TAG_ALGORITHM => s.algorithm = v.first().copied().unwrap_or(ALG_SHA1),
            _ => {}
        }
    }
    s.password_set = !s.challenge.is_empty();
    if s.password_set {
        let key = *s.access_key.as_ref().ok_or("password required")?;
        validate(s, &key)?;
    }
    Ok(())
}
fn validate(s: &OathSession, key: &[u8]) -> R<()> {
    let ours: [u8; 8] = rand::random();
    let mut body = tlv(TAG_RESPONSE_FULL, &hmac_alg(s.algorithm, key, &s.challenge));
    body.extend(tlv(TAG_CHALLENGE, &ours));
    let (d, sw) = transmit(&*s, &apdu(INS_VALIDATE, 0, 0, &body))?;
    if sw != 0x9000 {
        return Err(if sw == 0x6982 || sw == 0x6a80 || (sw & 0xfff0) == 0x63c0 { "wrong password".into() } else { sw_err("VALIDATE", sw) });
    }
    let proof = parse_tlvs(&d).into_iter().find(|(t, _)| *t == TAG_RESPONSE_FULL).map(|(_, v)| v).ok_or("VALIDATE returned no proof")?;
    if proof != hmac_alg(s.algorithm, key, &ours) {
        return Err("the key failed mutual authentication".into());
    }
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct OathInfo {
    pub reader: String,
    pub version: String,
    pub device_id: String,
    pub password_set: bool,
    pub unlocked: bool,
}
fn info_of(s: &OathSession) -> OathInfo {
    OathInfo {
        reader: s.reader.clone(),
        version: s.version.clone(),
        device_id: hex::encode(&s.device_id),
        password_set: s.password_set,
        unlocked: !s.password_set || s.access_key.is_some(),
    }
}

#[tauri::command]
pub async fn oath_connect(state: State<'_, OathState>, reader: String) -> R<OathInfo> {
    let ctx = Context::establish(Scope::User).map_err(|e| format!("PC/SC: {e}"))?;
    let cname = std::ffi::CString::new(reader.as_str()).map_err(|e| e.to_string())?;
    let card = ctx.connect(&cname, ShareMode::Shared, Protocols::ANY).map_err(|e| format!("connect to reader: {e}"))?;
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    *s = OathSession::default();
    s.card = Some(card);
    s._ctx = Some(ctx);
    s.reader = reader;
    // first select without auth: learn whether a password is set
    let r = {
        let mut a = vec![0x00, 0xa4, 0x04, 0x00, AID.len() as u8];
        a.extend_from_slice(&AID);
        transmit(&*s, &a)
    };
    match r {
        Ok((d, 0x9000)) => {
            for (t, v) in parse_tlvs(&d) {
                match t {
                    TAG_VERSION => s.version = v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."),
                    TAG_NAME => s.device_id = v,
                    TAG_CHALLENGE => s.challenge = v,
                    TAG_ALGORITHM => s.algorithm = v.first().copied().unwrap_or(ALG_SHA1),
                    _ => {}
                }
            }
            s.password_set = !s.challenge.is_empty();
        }
        Ok((_, 0x6a82)) => { *s = OathSession::default(); return Err("this key has no OATH applet".into()); }
        Ok((_, sw)) => { *s = OathSession::default(); return Err(sw_err("SELECT OATH", sw)); }
        Err(e) => { *s = OathSession::default(); return Err(e); }
    }
    Ok(info_of(&s))
}

/// Connect the OATH applet over the HID CTAPHID tunnel (HID-only Token2 keys).
/// Same protocol as PC/SC; only the transport differs. `path` is the HID device path.
#[tauri::command]
pub async fn oath_connect_hid(state: State<'_, OathState>, path: String) -> R<OathInfo> {
    let hid = crate::t2otp_hid::OtpHid::open(&path).map_err(|e| format!("open HID: {e}"))?;
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    *s = OathSession::default();
    s.hid = Some(hid);
    s.reader = path;
    let s = &mut *s;
    let r = {
        let mut a = vec![0x00, 0xa4, 0x04, 0x00, AID.len() as u8];
        a.extend_from_slice(&AID);
        transmit(&*s, &a)
    };
    match r {
        Ok((d, 0x9000)) => {
            for (t, v) in parse_tlvs(&d) {
                match t {
                    TAG_VERSION => s.version = v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."),
                    TAG_NAME => s.device_id = v,
                    TAG_CHALLENGE => s.challenge = v,
                    TAG_ALGORITHM => s.algorithm = v.first().copied().unwrap_or(ALG_SHA1),
                    _ => {}
                }
            }
            s.password_set = !s.challenge.is_empty();
        }
        Ok((_, 0x6a82)) => { *s = OathSession::default(); return Err("this key has no OATH applet".into()); }
        Ok((_, sw)) => { *s = OathSession::default(); return Err(sw_err("SELECT OATH", sw)); }
        Err(e) => { *s = OathSession::default(); return Err(e); }
    }
    Ok(info_of(s))
}

#[tauri::command]
pub fn oath_disconnect(state: State<OathState>) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    *s = OathSession::default();
    Ok(())
}

#[tauri::command]
pub async fn oath_unlock(state: State<'_, OathState>, password: String) -> R<OathInfo> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let key = derive_access_key(&password, &s.device_id.clone());
    s.access_key = Some(key);
    if let Err(e) = select_and_auth(&mut s) {
        s.access_key = None;
        return Err(e);
    }
    Ok(info_of(&s))
}

#[derive(Serialize, Clone)]
pub struct OathCredential {
    pub name: String,   // raw applet name, e.g. "Issuer:account" or "60/Issuer:account"
    pub issuer: String,
    pub account: String,
    pub kind: String,   // "totp" | "hotp"
    pub algorithm: String,
    pub period: u32,
    pub touch: bool,
    pub code: String,   // filled by calculate_all for TOTP (empty if touch/HOTP)
}

fn split_name(raw: &str) -> (String, String, u32) {
    let (period, rest) = match raw.split_once('/') {
        Some((p, r)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (p.parse().unwrap_or(30), r),
        _ => (30, raw),
    };
    let (issuer, account) = match rest.split_once(':') {
        Some((i, a)) => (i.to_string(), a.to_string()),
        None => (String::new(), rest.to_string()),
    };
    (issuer, account, period)
}

/// List credentials and compute all TOTP codes for the current time step.
#[tauri::command]
pub async fn oath_list(state: State<'_, OathState>) -> R<Vec<OathCredential>> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_and_auth(&mut s)?;
    let (d, sw) = transmit(&*s, &apdu(INS_LIST, 0, 0, &[]))?;
    if sw != 0x9000 {
        return Err(sw_err("LIST", sw));
    }
    let mut creds: Vec<OathCredential> = Vec::new();
    for (t, v) in parse_tlvs(&d) {
        if t == TAG_NAME_LIST && !v.is_empty() {
            let ta = v[0];
            let name = String::from_utf8_lossy(&v[1..]).into_owned();
            let (issuer, account, period) = split_name(&name);
            creds.push(OathCredential {
                name,
                issuer,
                account,
                kind: if ta & 0xf0 == TYPE_HOTP { "hotp".into() } else { "totp".into() },
                algorithm: match ta & 0x0f { ALG_SHA256 => "SHA256", ALG_SHA512 => "SHA512", _ => "SHA1" }.into(),
                period,
                touch: false,
                code: String::new(),
            });
        }
    }
    // CALCULATE ALL with the 30 s time step; 60 s credentials are computed individually
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let (d, sw) = transmit(&*s, &apdu(INS_CALCULATE_ALL, 0, 0x01, &tlv(TAG_CHALLENGE, &(now / 30).to_be_bytes())))?;
    if sw == 0x9000 {
        let tl = parse_tlvs(&d);
        let mut i = 0;
        while i + 1 < tl.len() {
            let (_, name) = &tl[i];
            let (rt, rv) = &tl[i + 1];
            let name = String::from_utf8_lossy(name).into_owned();
            if let Some(c) = creds.iter_mut().find(|c| c.name == name) {
                match *rt {
                    TAG_RESPONSE_TRUNC => c.code = decode_code(rv),
                    TAG_TOUCH => c.touch = true,
                    TAG_NO_RESPONSE => {} // HOTP: computed on demand
                    _ => {}
                }
            }
            i += 2;
        }
    }
    for c in creds.iter_mut() {
        if c.kind == "totp" && c.period != 30 && !c.touch {
            if let Ok(code) = calc_one(&*s, &c.name, Some(now / c.period as u64)) {
                c.code = code;
            }
        }
    }
    Ok(creds)
}

fn decode_code(v: &[u8]) -> String {
    if v.len() < 5 {
        return String::new();
    }
    let digits = v[0] as usize;
    let n = u32::from_be_bytes([v[1] & 0x7f, v[2], v[3], v[4]]);
    let m = 10u32.pow(digits.min(8) as u32);
    format!("{:0width$}", n % m, width = digits)
}
fn calc_one(s: &OathSession, name: &str, step: Option<u64>) -> R<String> {
    let mut body = tlv(TAG_NAME, name.as_bytes());
    body.extend(tlv(TAG_CHALLENGE, &step.unwrap_or(0).to_be_bytes()));
    let (d, sw) = transmit(&*s, &apdu(INS_CALCULATE, 0, 0x01, &body))?;
    if sw != 0x9000 {
        return Err(sw_err("CALCULATE", sw));
    }
    parse_tlvs(&d).into_iter().find(|(t, _)| *t == TAG_RESPONSE_TRUNC).map(|(_, v)| decode_code(&v)).ok_or("no code in response".into())
}

/// Compute one code now (HOTP advances the counter; touch credentials prompt).
#[tauri::command]
pub async fn oath_calculate(state: State<'_, OathState>, name: String, kind: String, period: u32) -> R<String> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_and_auth(&mut s)?;
    let step = if kind == "hotp" { None } else {
        Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs() / period.max(1) as u64)
    };
    calc_one(&*s, &name, step)
}

#[tauri::command]
pub async fn oath_put(
    state: State<'_, OathState>,
    issuer: String,
    account: String,
    secret_base32: String,
    kind: String,
    algorithm: String,
    digits: u8,
    period: u32,
    counter: u64,
    touch: bool,
) -> R<()> {
    let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret_base32.replace(' ', "").to_uppercase())
        .ok_or("secret is not valid base32")?;
    if secret.is_empty() {
        return Err("secret is empty".into());
    }
    if !(6..=8).contains(&digits) {
        return Err("digits must be 6–8".into());
    }
    let mut name = String::new();
    if kind == "totp" && period != 30 {
        name.push_str(&format!("{period}/"));
    }
    if !issuer.is_empty() {
        name.push_str(&issuer);
        name.push(':');
    }
    name.push_str(&account);
    if name.len() > 64 {
        return Err("issuer + account must be at most 64 bytes".into());
    }
    let ta = (if kind == "hotp" { TYPE_HOTP } else { TYPE_TOTP })
        | match algorithm.as_str() { "SHA256" => ALG_SHA256, "SHA512" => ALG_SHA512, _ => ALG_SHA1 };
    let mut key = vec![ta, digits];
    let mut sec = secret;
    if sec.len() < MIN_KEY {
        sec.resize(MIN_KEY, 0);
    }
    key.extend(sec);
    let mut body = tlv(TAG_NAME, name.as_bytes());
    body.extend(tlv(TAG_KEY, &key));
    if touch {
        body.extend(tlv(TAG_PROPERTY, &[PROP_TOUCH]));
    }
    if kind == "hotp" && counter > 0 {
        body.extend(tlv(0x7a, &(counter as u32).to_be_bytes())); // IMF tag
    }
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_and_auth(&mut s)?;
    let (_, sw) = transmit(&*s, &apdu(INS_PUT, 0, 0, &body))?;
    if sw != 0x9000 {
        return Err(sw_err("PUT", sw));
    }
    Ok(())
}

#[tauri::command]
pub async fn oath_delete(state: State<'_, OathState>, name: String) -> R<()> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_and_auth(&mut s)?;
    let (_, sw) = transmit(&*s, &apdu(INS_DELETE, 0, 0, &tlv(TAG_NAME, name.as_bytes())))?;
    if sw != 0x9000 {
        return Err(sw_err("DELETE", sw));
    }
    Ok(())
}

/// Set (or, with an empty password, remove) the applet password.
#[tauri::command]
pub async fn oath_set_password(state: State<'_, OathState>, password: String) -> R<OathInfo> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    select_and_auth(&mut s)?;
    let body = if password.is_empty() {
        tlv(TAG_KEY, &[])
    } else {
        let key = derive_access_key(&password, &s.device_id);
        let ch: [u8; 8] = rand::random();
        let mut b = tlv(TAG_KEY, &[&[TYPE_TOTP | ALG_SHA1][..], &key[..]].concat());
        b.extend(tlv(TAG_CHALLENGE, &ch));
        b.extend(tlv(TAG_RESPONSE_FULL, &hmac_alg(ALG_SHA1, &key, &ch)));
        b
    };
    let (_, sw) = transmit(&*s, &apdu(INS_SET_CODE, 0, 0, &body))?;
    if sw != 0x9000 {
        return Err(sw_err("SET CODE", sw));
    }
    s.access_key = if password.is_empty() { None } else { Some(derive_access_key(&password, &s.device_id)) };
    // re-select to pick up the new state
    let _ = select_and_auth(&mut s);
    Ok(info_of(&s))
}

/// Erase all credentials and the password.
#[tauri::command]
pub async fn oath_reset(state: State<'_, OathState>) -> R<OathInfo> {
    let mut s = state.0.lock().map_err(|_| "state lock poisoned")?;
    let mut a = vec![0x00, 0xa4, 0x04, 0x00, AID.len() as u8];
    a.extend_from_slice(&AID);
    let _ = transmit(&*s, &a)?;
    let (_, sw) = transmit(&*s, &[0x00, INS_RESET, 0xde, 0xad])?;
    if sw != 0x9000 {
        return Err(sw_err("RESET", sw));
    }
    s.access_key = None;
    let _ = select_and_auth(&mut s);
    Ok(info_of(&s))
}

/// Parse an otpauth:// URI into the fields the add dialog uses.
#[derive(Serialize)]
pub struct OtpAuth {
    pub kind: String,
    pub issuer: String,
    pub account: String,
    pub secret: String,
    pub algorithm: String,
    pub digits: u8,
    pub period: u32,
    pub counter: u64,
}
#[tauri::command]
pub fn oath_parse_uri(uri: String) -> R<OtpAuth> {
    let u = url::Url::parse(uri.trim()).map_err(|e| format!("not a valid otpauth URI: {e}"))?;
    if u.scheme() != "otpauth" {
        return Err("not an otpauth:// URI".into());
    }
    let kind = u.host_str().unwrap_or("totp").to_lowercase();
    let label = percent_decode(u.path().trim_start_matches('/'));
    let (mut issuer, account) = match label.split_once(':') {
        Some((i, a)) => (i.trim().to_string(), a.trim().to_string()),
        None => (String::new(), label.clone()),
    };
    let mut o = OtpAuth { kind, issuer: String::new(), account, secret: String::new(), algorithm: "SHA1".into(), digits: 6, period: 30, counter: 0 };
    for (k, v) in u.query_pairs() {
        match &*k {
            "secret" => o.secret = v.into_owned(),
            "issuer" => issuer = v.into_owned(),
            "algorithm" => o.algorithm = v.to_uppercase().replace('-', ""),
            "digits" => o.digits = v.parse().unwrap_or(6),
            "period" => o.period = v.parse().unwrap_or(30),
            "counter" => o.counter = v.parse().unwrap_or(0),
            _ => {}
        }
    }
    o.issuer = issuer;
    if o.secret.is_empty() {
        return Err("URI has no secret".into());
    }
    Ok(o)
}
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
pub fn oath_list_readers() -> R<Vec<String>> {
    let ctx = Context::establish(Scope::User).map_err(|e| format!("PC/SC: {e}"))?;
    let mut buf = vec![0u8; 4096];
    let names = ctx.list_readers(&mut buf).map_err(|e| format!("list readers: {e}"))?;
    Ok(names.map(|n| n.to_string_lossy().into_owned()).collect())
}
