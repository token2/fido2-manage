//! Token2 PIN+ serial-number prefix reference → firmware generation, model,
//! branding. Source: token2.com PIN+ firmware feature support matrix,
//! "PIN+ Serial Number Prefix Reference". Prefix = first 5 digits of the
//! 12-digit serial; the rest is a check digit and a sequence.

#[derive(serde::Serialize, Clone, Debug, Default)]
pub struct T2Model {
    pub revision: String, // R1, R2, R3, R3.1, R3.2, R3.3, R3.4
    pub model: String,
    pub branding: String,
    pub piv: bool,
    pub otp_protection: bool,
    pub bio: bool,
}

const TABLE: &[(&str, &str, &str, &str)] = &[
    // prefix, revision, model, branding
    ("86105", "R1", "USB-A NFC", "Token2"),
    ("86104", "R1", "USB-C NFC", "Token2"),
    ("86103", "R1", "Dual NFC", "Token2"),
    ("86202", "R1", "FIDO Card", "Token2"),
    ("96105", "R2", "USB-A PIN+ NFC", "Token2"),
    ("96104", "R2", "USB-C PIN+ NFC", "Token2"),
    ("96103", "R2", "Dual PIN+ NFC", "Token2"),
    ("23103", "R2", "Dual PIN+ NFC", "unbranded"),
    ("76103", "R3", "Dual PIN+ NFC", "Token2"),
    ("76104", "R3", "USB-C PIN+ NFC", "Token2"),
    ("76202", "R3", "FIDO Card", "Token2"),
    ("86106", "R3", "FIDO Card (no 7816 contact)", "unbranded"),
    ("76106", "R3", "FIDO Card (7816 contact)", "unbranded"),
    ("76105", "R3.1", "USB-A PIN+ NFC", "Token2"),
    ("26105", "R3.1", "USB-A PIN+ NFC", "unbranded"),
    ("72102", "R3.1", "Mini USB-C PIN+", "Token2"),
    ("77103", "R3.2", "Dual PIN+ NFC", "Token2"),
    ("24103", "R3.2", "Dual PIN+ NFC", "unbranded"),
    ("72101", "R3.2", "Mini USB-A PIN+", "Token2"),
    ("72103", "R3.2", "Bio3 Dual A+C PIN+", "Token2"),
    ("22103", "R3.2", "Bio3 Dual A+C PIN+", "unbranded"),
    ("66105", "R3.3", "USB-A NFC PIN+ PIV+", "Token2"),
    ("66104", "R3.3", "USB-C NFC PIN+ PIV+", "Token2"),
    ("66103", "R3.3", "Dual NFC PIN+ PIV+", "Token2"),
    ("66107", "R3.3", "USB-A NFC PIN+ PIV+", "unbranded"),
    ("66106", "R3.3", "USB-C NFC PIN+ PIV+", "unbranded"),
    ("66114", "R3.3", "Dual NFC PIN+ PIV+", "unbranded"),
    ("66113", "R3.3", "Dual NFC PIN+ PIV+ Octo", "unbranded"),
    ("66202", "R3.3", "FIDO Card NFC+7816 PIN+ PIV+", "Token2"),
    ("66102", "R3.3", "FIDO Card PIN+ PIV+", "unbranded (white)"),
    ("66302", "R3.3", "FIDO Card NFC+7816 PIN+ PIV+", "unbranded (white)"),
    ("66101", "R3.3", "Mini USB-A PIN+ PIV+", "Token2"),
    ("66111", "R3.3", "Mini USB-C PIN+ PIV+", "Token2"),
    ("72113", "R3.3", "Dual Bio3 PIN+ PIV+", "Token2"),
    ("24133", "R3.3", "Dual Bio3 PIN+ PIV+", "unbranded"),
    ("65103", "R3.4", "PIN+ Dual Ace PIV+", "Token2"),
    ("65104", "R3.4", "PIN+ Dual Ace PIV+", "unbranded"),
    ("72114", "R3.4", "Dual Bio3 PIN+ PIV+", "Token2"),
    ("65202", "R3.4", "FIDO Card NFC+7816 PIN+ PIV+", "Token2"),
    ("65102", "R3.4", "FIDO Card NFC PIN+ PIV+", "unbranded"),
    ("65302", "R3.4", "FIDO Card NFC+7816 PIN+ PIV+", "unbranded"),
    ("65101", "R3.4", "Mini USB-A PIN+ PIV+", "Token2"),
    ("65111", "R3.4", "Mini USB-C PIN+ PIV+", "Token2"),
];

pub fn from_serial(serial: &str) -> Option<T2Model> {
    let digits: String = serial.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.starts_with("7000") && digits.len() >= 8 {
        return Some(T2Model { revision: "R3.1".into(), model: "Custom system access card (contact 7816 + NFC)".into(), branding: "special".into(), ..Default::default() });
    }
    if digits.len() < 5 {
        return None;
    }
    let p = &digits[..5];
    TABLE.iter().find(|(pre, ..)| *pre == p).map(|(_, rev, model, brand)| T2Model {
        revision: rev.to_string(),
        model: model.to_string(),
        branding: brand.to_string(),
        piv: matches!(*rev, "R3.3" | "R3.4"),
        otp_protection: *rev == "R3.4",
        bio: model.contains("Bio3"),
    })
}
