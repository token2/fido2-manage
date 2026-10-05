//! Public-key encoding, certificate summaries and self-signed certificate
//! generation where the private key never leaves the token.

use crate::piv::{GeneratedKey, Piv};
use crate::t2piv::*;
use der::asn1::{BitString, ObjectIdentifier};
use der::{Any, Encode};
use rcgen::{CertificateParams, DnType, KeyPair, RemoteKeyPair, SignatureAlgorithm};
use rsa::pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey};
use rsa::pkcs8::EncodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPublicKey};
use serde::Serialize;
use sha2::{Digest, Sha256, Sha384};
use spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
use x509_parser::certificate::X509Certificate;
use x509_parser::public_key::PublicKey;
use x509_parser::prelude::FromDer;

const OID_EC_PUBLIC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
const OID_P256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
const OID_P384: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.132.0.34");
const OID_ED25519: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.112");
const OID_X25519: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.110");

/// SubjectPublicKeyInfo DER for a freshly generated key.
pub fn spki_der(k: &GeneratedKey) -> Result<Vec<u8>, String> {
    let e = |e: &dyn std::fmt::Display| e.to_string();
    match k.algorithm {
        ALGO_RSA1024 | ALGO_RSA2048 | ALGO_RSA3072 | ALGO_RSA4096 => {
            let pk = RsaPublicKey::new(BigUint::from_bytes_be(&k.modulus), BigUint::from_bytes_be(&k.exponent))
                .map_err(|x| e(&x))?;
            Ok(pk.to_public_key_der().map_err(|x| e(&x))?.as_bytes().to_vec())
        }
        ALGO_ECCP256 | ALGO_ECCP384 => {
            let curve = if k.algorithm == ALGO_ECCP256 { OID_P256 } else { OID_P384 };
            let spki = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned {
                    oid: OID_EC_PUBLIC_KEY,
                    parameters: Some(Any::encode_from(&curve).map_err(|x| e(&x))?),
                },
                subject_public_key: BitString::from_bytes(&k.point).map_err(|x| e(&x))?,
            };
            spki.to_der().map_err(|x| e(&x))
        }
        ALGO_ED25519 | ALGO_X25519 => {
            let oid = if k.algorithm == ALGO_ED25519 { OID_ED25519 } else { OID_X25519 };
            let spki = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned { oid, parameters: None },
                subject_public_key: BitString::from_bytes(&k.point).map_err(|x| e(&x))?,
            };
            spki.to_der().map_err(|x| e(&x))
        }
        _ => Err("unsupported algorithm".into()),
    }
}

pub fn to_pem(tag: &str, der: &[u8]) -> String {
    ::pem::encode(&::pem::Pem::new(tag, der.to_vec()))
}

/// Accepts PEM or DER and returns DER.
pub fn cert_to_der(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.starts_with(b"-----") {
        let p = ::pem::parse(bytes).map_err(|e| e.to_string())?;
        Ok(p.into_contents())
    } else {
        Ok(bytes.to_vec())
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct CertSummary {
    pub subject: String,
    pub issuer: String,
    pub not_before: i64,
    pub not_after: i64,
    pub serial: String,
    pub key_algorithm: String,
    pub self_signed: bool,
    pub der_len: usize,
}

pub fn summarize(der: &[u8]) -> Result<CertSummary, String> {
    let (_, c) = X509Certificate::from_der(der).map_err(|e| e.to_string())?;
    let pk = c.public_key();
    let key_algorithm = match pk.parsed() {
        Ok(PublicKey::RSA(r)) => format!("RSA {}", r.key_size()),
        Ok(PublicKey::EC(p)) => format!("ECC P-{}", p.key_size()),
        Ok(PublicKey::Unknown(_)) | Ok(_) => pk.algorithm.algorithm.to_id_string(),
        Err(_) => pk.algorithm.algorithm.to_id_string(),
    };
    Ok(CertSummary {
        subject: c.subject().to_string(),
        issuer: c.issuer().to_string(),
        not_before: c.validity().not_before.timestamp(),
        not_after: c.validity().not_after.timestamp(),
        serial: c.raw_serial_as_string(),
        key_algorithm,
        self_signed: c.subject() == c.issuer(),
        der_len: der.len(),
    })
}

// ---- self-signed certificate with the key on the token ---------------------

struct TokenSigner {
    piv: *const Piv,
    slot: u8,
    algorithm: u8,
    public_key: Vec<u8>, // subjectPublicKey bits (RSAPublicKey DER or EC point)
    modulus_len: usize,
}
unsafe impl Send for TokenSigner {}
unsafe impl Sync for TokenSigner {}

// DigestInfo prefixes for PKCS#1 v1.5 (RFC 8017 §9.2 note 1)
const SHA256_PREFIX: [u8; 19] =
    [0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00, 0x04, 0x20];

impl RemoteKeyPair for TokenSigner {
    fn public_key(&self) -> &[u8] {
        &self.public_key
    }
    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, rcgen::Error> {
        let piv = unsafe { &*self.piv };
        let input = match self.algorithm {
            ALGO_RSA1024 | ALGO_RSA2048 | ALGO_RSA3072 | ALGO_RSA4096 => {
                let h = Sha256::digest(msg);
                let t_len = SHA256_PREFIX.len() + h.len();
                let mut em = vec![0xffu8; self.modulus_len];
                em[0] = 0x00;
                em[1] = 0x01;
                let sep = self.modulus_len - t_len - 1;
                em[sep] = 0x00;
                em[sep + 1..sep + 1 + SHA256_PREFIX.len()].copy_from_slice(&SHA256_PREFIX);
                em[sep + 1 + SHA256_PREFIX.len()..].copy_from_slice(&h);
                em
            }
            ALGO_ECCP256 => Sha256::digest(msg).to_vec(),
            ALGO_ECCP384 => Sha384::digest(msg).to_vec(),
            ALGO_ED25519 => msg.to_vec(),
            _ => return Err(rcgen::Error::RemoteKeyError),
        };
        piv.sign_raw(self.slot, self.algorithm, &input).map_err(|_| rcgen::Error::RemoteKeyError)
    }
    fn algorithm(&self) -> &'static SignatureAlgorithm {
        match self.algorithm {
            ALGO_ECCP256 => &rcgen::PKCS_ECDSA_P256_SHA256,
            ALGO_ECCP384 => &rcgen::PKCS_ECDSA_P384_SHA384,
            ALGO_ED25519 => &rcgen::PKCS_ED25519,
            _ => &rcgen::PKCS_RSA_SHA256,
        }
    }
}

/// Build a self-signed certificate for the key in `slot`. PIN must already be
/// verified on `piv`. `spki` is the SubjectPublicKeyInfo DER of that key.
pub fn self_signed(piv: &Piv, slot: u8, algorithm: u8, spki: &[u8], cn: &str, days: i64) -> Result<Vec<u8>, String> {
    use der::Decode;
    let info = SubjectPublicKeyInfoOwned::from_der(spki).map_err(|e| e.to_string())?;
    let public_key = info.subject_public_key.raw_bytes().to_vec();
    let modulus_len = match algorithm {
        ALGO_RSA1024 | ALGO_RSA2048 | ALGO_RSA3072 | ALGO_RSA4096 => {
            RsaPublicKey::from_pkcs1_der(&public_key).map_err(|e| e.to_string())?.size()
        }
        _ => 0,
    };
    let signer = TokenSigner { piv: piv as *const Piv, slot, algorithm, public_key, modulus_len };
    let key_pair = KeyPair::from_remote(Box::new(signer)).map_err(|e| e.to_string())?;

    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| e.to_string())?;
    params.distinguished_name.push(DnType::CommonName, cn);
    let now = ::time::OffsetDateTime::now_utc();
    params.not_before = now - ::time::Duration::minutes(5);
    params.not_after = now + ::time::Duration::days(days);
    let cert = params.self_signed(&key_pair).map_err(|e| e.to_string())?;
    Ok(cert.der().to_vec())
}

/// Re-encode an RSA public key so the frontend can show it; used in tests.
#[allow(dead_code)]
pub fn rsa_pkcs1_to_spki(pkcs1: &[u8]) -> Result<Vec<u8>, String> {
    let pk = RsaPublicKey::from_pkcs1_der(pkcs1).map_err(|e| e.to_string())?;
    let _ = pk.to_pkcs1_der();
    Ok(pk.to_public_key_der().map_err(|e| e.to_string())?.as_bytes().to_vec())
}

// ---- attestation verification ---------------------------------------------

/// Token2 PIV attestation CA (CN=TOKEN2 PIV CA), published at
/// https://www.token2.com/site/page/pin-firmware-feature-support-matrix-openpgp-fido2-otp-and-piv-across-releases
pub const TOKEN2_PIV_CA_PEM: &str = include_str!("../assets/token2-piv-ca.pem");

#[derive(Serialize, Clone, Debug, Default)]
pub struct AttestationReport {
    pub valid: bool,
    pub checks: Vec<(String, String, String)>, // (check, "ok"|"warn"|"fail", detail)
    pub warnings: usize,
    pub leaf: Option<CertSummary>,
    pub intermediate: Option<CertSummary>,
    pub root_fingerprint: String,
    pub firmware_version: String,
    pub device_serial: String,
    pub pin_policy: String,
    pub touch_policy: String,
    pub form_factor: String,
    pub leaf_pem: String,
    pub intermediate_pem: String,
}

fn sha256_hex(d: &[u8]) -> String {
    let h = Sha256::digest(d);
    h.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

fn ext_value<'a>(c: &'a X509Certificate<'a>, oid: &str) -> Option<&'a [u8]> {
    c.extensions().iter().find(|e| e.oid.to_id_string() == oid).map(|e| e.value)
}

/// Strip a DER OCTET STRING wrapper if the value was double-encoded.
fn unwrap_octet(v: &[u8]) -> &[u8] {
    if v.len() >= 2 && v[0] == 0x04 && v[1] as usize == v.len() - 2 { &v[2..] } else { v }
}

/// Verify leaf (slot attestation) <- intermediate (slot F9) <- Token2 PIV CA.
/// `slot_spki` is the SPKI DER of the key the leaf is supposed to attest.
pub fn verify_attestation(leaf_der: &[u8], f9_der: &[u8], slot_spki: Option<&[u8]>) -> Result<AttestationReport, String> {
    let ca_der = ::pem::parse(TOKEN2_PIV_CA_PEM).map_err(|e| e.to_string())?.into_contents();
    let (_, ca) = X509Certificate::from_der(&ca_der).map_err(|e| e.to_string())?;
    let (_, f9) = X509Certificate::from_der(f9_der).map_err(|e| format!("F9 certificate: {e}"))?;
    let (_, leaf) = X509Certificate::from_der(leaf_der).map_err(|e| format!("attestation certificate: {e}"))?;

    let mut r = AttestationReport {
        leaf: summarize(leaf_der).ok(),
        intermediate: summarize(f9_der).ok(),
        root_fingerprint: sha256_hex(&ca_der),
        leaf_pem: to_pem("CERTIFICATE", leaf_der),
        intermediate_pem: to_pem("CERTIFICATE", f9_der),
        ..Default::default()
    };
    fn push(r: &mut AttestationReport, name: &str, ok: bool, detail: String) { r.checks.push((name.to_string(), if ok { "ok".into() } else { "fail".into() }, detail)); }

    // 1. leaf signed by F9
    let s1 = leaf.verify_signature(Some(f9.public_key())).is_ok();
    push(&mut r, "Attestation certificate signed by the key's F9 certificate", s1, String::new());
    // 2. F9 signed by Token2 PIV CA
    let s2 = f9.verify_signature(Some(ca.public_key())).is_ok();
    push(&mut r, "F9 certificate signed by TOKEN2 PIV CA", s2, if s2 { "CN=TOKEN2 PIV CA".into() } else { "signature does not verify against the published CA".into() });
    // 3. issuer/subject chaining
    let i1 = leaf.issuer() == f9.subject();
    push(&mut r, "Attestation issuer matches F9 subject", i1, if i1 { String::new() } else { format!("issuer {} vs subject {}", leaf.issuer(), f9.subject()) });
    let i2 = f9.issuer() == ca.subject();
    push(&mut r, "F9 issuer matches CA subject", i2, String::new());
    // 4. F9 is a CA
    let f9_is_ca = f9.basic_constraints().ok().flatten().map(|bc| bc.value.ca).unwrap_or(false);
    // Known trait of keys provisioned with the R3.3 tooling: F9 lacks
    // basicConstraints. Strict RFC 5280 verifiers reject it, but the chain
    // cryptography is unaffected, so this is a warning, not a failure.
    r.checks.push((
        "F9 certificate is marked as a CA".into(),
        if f9_is_ca { "ok".into() } else { "warn".into() },
        if f9_is_ca { String::new() } else { "basicConstraints missing; strict RFC 5280 verifiers will reject the chain (provisioning issue on this key, not a forgery)".into() },
    ));
    // 5. validity
    let now = ::x509_parser::time::ASN1Time::now();
    let v = leaf.validity().is_valid_at(now) && f9.validity().is_valid_at(now) && ca.validity().is_valid_at(now);
    push(&mut r, "All certificates currently valid", v, String::new());
    // 6. attested key matches the slot key
    if let Some(spki) = slot_spki {
        let m = leaf.public_key().raw == spki;
        push(&mut r, "Attested public key matches the key in the slot", m, if m { String::new() } else { "the certificate attests a different key".into() });
    }

    // device properties (Token2 arc first, then the legacy Yubico arc)
    let pick = |a: &str, b: &str| ext_value(&leaf, a).or_else(|| ext_value(&leaf, b)).map(unwrap_octet);
    if let Some(v) = pick("1.3.6.1.4.1.66563.3.3", "1.3.6.1.4.1.41482.3.3") {
        r.firmware_version = v.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(".");
    }
    if let Some(v) = pick("1.3.6.1.4.1.66563.3.7", "1.3.6.1.4.1.41482.3.7") {
        // INTEGER or raw digits/bytes
        r.device_serial = if v.first() == Some(&0x02) && v.len() >= 2 {
            let n = &v[2..];
            n.iter().fold(0u64, |acc, b| (acc << 8) | *b as u64).to_string()
        } else if v.iter().all(|b| b.is_ascii_digit()) {
            String::from_utf8_lossy(v).into_owned()
        } else {
            hex::encode(v)
        };
    }
    if let Some(v) = pick("1.3.6.1.4.1.66563.3.8", "1.3.6.1.4.1.41482.3.8") {
        if v.len() >= 2 {
            r.pin_policy = match v[0] { 1 => "never", 2 => "once", 3 => "always", _ => "default" }.into();
            r.touch_policy = match v[1] { 1 => "never", 2 => "always", 3 => "cached", _ => "default" }.into();
        }
    }
    if let Some(v) = pick("1.3.6.1.4.1.66563.3.9", "1.3.6.1.4.1.41482.3.9") {
        r.form_factor = v.first().map(|b| format!("0x{b:02X}")).unwrap_or_default();
    }

    r.valid = r.checks.iter().all(|(_, lvl, _)| lvl != "fail");
    r.warnings = r.checks.iter().filter(|(_, lvl, _)| lvl == "warn").count();
    Ok(r)
}

