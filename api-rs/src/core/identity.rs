//! Keyed HMACs of identifiers, `reporter_id` and `ip_prefix` (the connection's /24 or /48 when
//! absent). Keys come from an environment secret; raw values are never stored.

use super::pytext;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::net::IpAddr;

pub const MIN_KEY_LENGTH: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub struct KeyTooShort;

impl std::fmt::Display for KeyTooShort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SCAM_MAP_HMAC_KEY must be at least {MIN_KEY_LENGTH} characters"
        )
    }
}

impl std::error::Error for KeyTooShort {}

/// Turns raw values into keyed digests. Each purpose has its own domain, so the same text as an
/// identifier and as a `reporter_id` gives unrelated digests.
#[derive(Clone)]
pub struct Keyring {
    key: Vec<u8>,
}

impl std::fmt::Debug for Keyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Keyring(..)")
    }
}

impl Keyring {
    /// # Errors
    /// A key shorter than [`MIN_KEY_LENGTH`] characters.
    pub fn new(key: &str) -> Result<Self, KeyTooShort> {
        if key.chars().count() < MIN_KEY_LENGTH {
            return Err(KeyTooShort);
        }
        Ok(Self {
            key: key.as_bytes().to_vec(),
        })
    }

    fn mac(&self, message: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC takes any key length");
        mac.update(message);
        mac.finalize().into_bytes().to_vec()
    }

    pub fn digest(&self, purpose: &str, value: &str) -> String {
        hex::encode(self.mac(format!("{purpose}\0{value}").as_bytes()))
    }

    pub fn identifier(&self, kind: &str, normalised: &str) -> String {
        self.digest(&format!("id:{kind}"), normalised)
    }

    pub fn reporter(&self, reporter_id: &str) -> String {
        self.digest("reporter", pytext::strip(reporter_id))
    }

    pub fn ip_prefix(&self, prefix: &str) -> String {
        self.digest("ip", pytext::strip(prefix))
    }

    /// A separate key for note fingerprints, so sketches are useless without the secret.
    pub fn note_key(&self) -> Vec<u8> {
        self.mac(b"notes")
    }
}

/// The /24 of an IPv4 address or the /48 of an IPv6 address; never the full address.
pub fn connection_prefix(host: Option<&str>) -> String {
    match host.and_then(|h| h.parse::<IpAddr>().ok()) {
        Some(IpAddr::V4(v4)) => {
            let [a, b, c, _] = v4.octets();
            format!("{a}.{b}.{c}.0")
        }
        Some(IpAddr::V6(v6)) => {
            let mut segments = v6.segments();
            segments[3..].fill(0);
            std::net::Ipv6Addr::from(segments).to_string()
        }
        None => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "test-key-0123456789";

    #[test]
    fn refuses_a_short_key() {
        assert!(Keyring::new("short").is_err());
        assert!(Keyring::new(KEY).is_ok());
    }

    /// Values computed with the Python Keyring (`api/app/core/identity.py`) and the same key.
    #[test]
    fn digests_match_the_python_api() {
        let k = Keyring::new(KEY).unwrap();
        assert_eq!(
            k.identifier("phone", "+919876543210"),
            "bed1a6a156b2d3445be719fffaa98b93e6e8fc9f10f8126ea8d7817ea1f08515"
        );
        assert_eq!(
            k.reporter(" r_1 "),
            "ef38217ee62baaeb7cffe02ef2c8e9d9a3b3c9ee6841604302bece663a481cdf"
        );
        assert_eq!(
            hex::encode(k.note_key()),
            "cf36366d7ae464e6027571eb380512166b560a4d9f825435e429d9dee57073ac"
        );
    }

    #[test]
    fn purposes_are_separate() {
        let k = Keyring::new(KEY).unwrap();
        assert_ne!(k.reporter("x"), k.ip_prefix("x"));
        assert_ne!(k.identifier("phone", "x"), k.identifier("upi", "x"));
    }

    #[test]
    fn connection_prefixes() {
        assert_eq!(connection_prefix(Some("203.0.113.77")), "203.0.113.0");
        assert_eq!(
            connection_prefix(Some("2001:db8:abcd:12::1")),
            "2001:db8:abcd::"
        );
        assert_eq!(connection_prefix(Some("::ffff:1.2.3.4")), "::");
        assert_eq!(connection_prefix(Some("2001:0:0:1::")), "2001::");
        assert_eq!(connection_prefix(Some("not an ip")), "unknown");
        assert_eq!(connection_prefix(None), "unknown");
    }
}
