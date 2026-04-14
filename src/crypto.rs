//! PIN derivation and V2 authentication payload construction.
//!
//! See `PROTOCOL.md` for the full algorithm specification.

use sha2::{Digest, Sha256};

use crate::protocol::SOLD_DATE_DEFAULT;

// Variant-2 lookup tables for the intermediate key derivation.

const HASH_IDX_V2: [u8; 16] = [
    0x19, 0x05, 0x07, 0x16, 0x13, 0x0c, 0x16, 0x0b, 0x0c, 0x05, 0x18, 0x0d, 0x17, 0x17, 0x12, 0x30,
];

const MIX_V2: [u16; 16] = [
    0xc356, 0x9b94, 0x0dbd, 0xc561, 0x3139, 0x300f, 0xee5a, 0xe883, 0x4638, 0x89ca, 0x8b30, 0x593d,
    0xe47d, 0x2ea2, 0x89bc, 0xaafe,
];

/// Normalize a raw sold-on date string.
///
/// Strips all non-digit characters, takes the first 8 digits, and falls
/// back to [`SOLD_DATE_DEFAULT`] if fewer than 8 digits remain.
pub fn normalize_sold_on(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        SOLD_DATE_DEFAULT.to_string()
    } else {
        digits[..8].to_string()
    }
}

fn sha256_vin_date(vin: &str, sold_on: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(vin.as_bytes());
    hasher.update(b"-");
    hasher.update(sold_on.as_bytes());
    hasher.finalize().into()
}

/// Compute the BLE pairing PIN for a given (VIN, sold_on).
///
/// Returns a 6-character zero-padded decimal string (e.g. `"001379"`).
/// The underlying numeric range is 0000–9999. The BLE SMP passkey is
/// the numeric value (e.g. `1379`).
///
/// The addition step MUST use 32-bit arithmetic, not 8-bit wrapping.
/// See `PROTOCOL.md` for details.
pub fn generate_pin(vin: &str, sold_on: &str) -> String {
    let h = sha256_vin_date(vin, sold_on);

    let d1 = ((h[15] ^ 0xB3) as u32 + 0xEF) % 10;
    let d2 = ((h[9] ^ 0x9E) as u32 + 0x64) % 10;
    let d3 = ((h[14] ^ 0xEC) as u32 + 0xD1) % 10;
    let d4 = ((h[3] ^ 0xC5) as u32 + 0xE4) % 10;

    let pin = d1 * 1000 + d2 * 100 + d3 * 10 + d4;
    format!("{:06}", pin)
}

/// Parse a PIN string (as returned by [`generate_pin`]) into the numeric
/// passkey value expected by BLE SMP.
pub fn pin_to_passkey(pin: &str) -> u32 {
    pin.parse().unwrap_or(0)
}

/// Derive the Wi-Fi password for the bike's firmware update AP.
///
/// The SSID is `VARG-<VIN>`. The password is the first 16 characters of
/// the uppercase hex SHA-256 of `"VIN-SOLDDATE"`.
pub fn wifi_password(vin: &str, sold_on: &str) -> String {
    let h = sha256_vin_date(vin, sold_on);
    let hex: String = h.iter().map(|b| format!("{:02X}", b)).collect();
    hex[..16].to_string()
}

/// The Wi-Fi SSID the bike broadcasts during firmware updates.
pub fn wifi_ssid(vin: &str) -> String {
    format!("VARG-{}", vin)
}

/// Derive the 16-byte intermediate key `K` from (VIN, sold_on) using the
/// V2 lookup tables.
///
/// Byte-level wrapping is correct here (only the low byte is stored).
fn derive_k_v2(vin: &str, sold_on: &str) -> [u8; 16] {
    let h = sha256_vin_date(vin, sold_on);
    let mut k = [0u8; 16];
    for i in 0..16usize {
        let idx = (HASH_IDX_V2[i] ^ (i as u8 + 0x11)) as usize;
        debug_assert!(idx < 32, "HASH_IDX_V2[{i}] produced oob index {idx}");
        let mix: u32 = (i as u32).wrapping_mul(0x1111) ^ (MIX_V2[i] as u32) ^ 0xFFFF_A5A5;
        let mix_hi = ((mix >> 8) & 0xff) as u8;
        let mix_lo = (mix & 0xff) as u8;
        k[i] = (h[idx] ^ mix_hi).wrapping_add(mix_lo);
    }
    k
}

/// Build the 34-byte V2 challenge-response payload.
///
/// Wire layout: `[0x02, 0x01] || SHA-256(K || [0x02, 0x01] || nonce)`.
pub fn build_auth_payload_v2(vin: &str, sold_on: &str, nonce: &[u8; 32]) -> [u8; 34] {
    let k = derive_k_v2(vin, sold_on);
    const HEADER: [u8; 2] = [0x02, 0x01];

    let mut hasher = Sha256::new();
    hasher.update(k);
    hasher.update(HEADER);
    hasher.update(nonce);
    let digest = hasher.finalize();

    let mut out = [0u8; 34];
    out[..2].copy_from_slice(&HEADER);
    out[2..].copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_is_always_six_digits() {
        let pin = generate_pin("ANYTHING", SOLD_DATE_DEFAULT);
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn pin_is_deterministic() {
        let a = generate_pin("STARKTEST12345", SOLD_DATE_DEFAULT);
        let b = generate_pin("STARKTEST12345", SOLD_DATE_DEFAULT);
        assert_eq!(a, b);
    }

    #[test]
    fn pin_depends_on_both_inputs() {
        let a = generate_pin("VINA", SOLD_DATE_DEFAULT);
        let b = generate_pin("VINB", SOLD_DATE_DEFAULT);
        assert_ne!(a, b, "different VINs must produce different PINs");

        let c = generate_pin("VINA", "20230101");
        assert_ne!(a, c, "different sold_on must produce different PINs");
    }

    #[test]
    fn pin_to_passkey_parses_correctly() {
        assert_eq!(pin_to_passkey("001379"), 1379);
        assert_eq!(pin_to_passkey("000000"), 0);
        assert_eq!(pin_to_passkey("009999"), 9999);
    }

    #[test]
    fn k_derivation_uses_in_range_indices() {
        for (i, &v) in HASH_IDX_V2.iter().enumerate() {
            let idx = v ^ (i as u8 + 0x11);
            assert!(idx < 32, "HASH_IDX_V2[{i}] -> idx {idx}");
        }
    }

    #[test]
    fn k_derivation_is_deterministic() {
        let a = derive_k_v2("SOMEVIN", SOLD_DATE_DEFAULT);
        let b = derive_k_v2("SOMEVIN", SOLD_DATE_DEFAULT);
        assert_eq!(a, b);
    }

    #[test]
    fn payload_shape_is_correct() {
        let nonce = [0xAAu8; 32];
        let payload = build_auth_payload_v2("VIN", SOLD_DATE_DEFAULT, &nonce);
        assert_eq!(payload.len(), 34);
        assert_eq!(&payload[..2], &[0x02, 0x01]);
    }

    #[test]
    fn payload_depends_on_nonce() {
        let a = build_auth_payload_v2("VIN", SOLD_DATE_DEFAULT, &[0u8; 32]);
        let b = build_auth_payload_v2("VIN", SOLD_DATE_DEFAULT, &[1u8; 32]);
        assert_ne!(&a[2..], &b[2..], "digest must change with nonce");
    }

    #[test]
    fn normalize_sold_on_handles_edge_cases() {
        assert_eq!(normalize_sold_on(""), SOLD_DATE_DEFAULT);
        assert_eq!(normalize_sold_on("abc"), SOLD_DATE_DEFAULT);
        assert_eq!(normalize_sold_on("1970"), SOLD_DATE_DEFAULT);
        assert_eq!(normalize_sold_on("19700101"), "19700101");
        assert_eq!(normalize_sold_on("2024-06-15"), "20240615");
        assert_eq!(normalize_sold_on("2024/06/15 12:00:00"), "20240615");
    }
}
