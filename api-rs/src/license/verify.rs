//! Ed25519 signature check over the canonical JSON of `licensee` and `expires_at`.
//!
//! Format: `{"licensee", "expires_at", "signature"}`. The signature is Ed25519 over the canonical
//! JSON of the other two fields (sorted keys, no spaces, UTF-8), base64-encoded. These are the
//! same bytes `scripts/license_sign.py` signs, so its files verify here unchanged.

use crate::core::isotime::{self, ParseError};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

/// The public key built into the binary; `SCAM_MAP_LICENSE_PUBLIC_KEY` can name another file.
const BUILT_IN_KEY: &str = include_str!("../../public_key.pem");

/// The licence file is malformed, unsigned, signed by another key, edited, or already expired.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidLicence(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceDoc {
    pub licensee: String,
    pub expires_at: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Licence {
    pub doc: LicenceDoc,
    pub expires_at: OffsetDateTime,
}

/// The exact bytes that are signed: Python `json.dumps(sort_keys=True, separators=(",", ":"),
/// ensure_ascii=False)` of the two fields. Python and `serde_json` escape strings the same way
/// once non-ASCII is kept as is.
///
/// # Panics
/// Never: a string always serialises.
pub fn canonical_bytes(licensee: &str, expires_at: &str) -> Vec<u8> {
    let quote = |s: &str| serde_json::to_string(s).expect("strings serialise");
    format!(
        "{{\"expires_at\":{},\"licensee\":{}}}",
        quote(expires_at),
        quote(licensee)
    )
    .into_bytes()
}

/// The configured public key: the file at `path` if given, else the built-in one. `None` when it
/// cannot be read or is not an Ed25519 key, so every licence is refused.
pub fn load_public_key(path: Option<&Path>) -> Option<VerifyingKey> {
    let pem = match path {
        Some(p) => std::fs::read_to_string(p).ok()?,
        None => BUILT_IN_KEY.to_owned(),
    };
    VerifyingKey::from_public_key_pem(pem.trim()).ok()
}

/// # Errors
/// No key, empty fields, a signature that is not base64 or does not match, or a bad time.
pub fn verify(doc: &LicenceDoc, key: Option<&VerifyingKey>) -> Result<Licence, InvalidLicence> {
    let key = key.ok_or(InvalidLicence(
        "no licence public key is configured on this server",
    ))?;
    if doc.licensee.is_empty() || doc.expires_at.is_empty() || doc.signature.is_empty() {
        return Err(InvalidLicence(
            "licensee, expires_at and signature must be non-empty strings",
        ));
    }
    let raw = STANDARD
        .decode(&doc.signature)
        .map_err(|_| InvalidLicence("signature is not base64"))?;
    let signature =
        Signature::from_slice(&raw).map_err(|_| InvalidLicence("signature does not match"))?;
    key.verify_strict(&canonical_bytes(&doc.licensee, &doc.expires_at), &signature)
        .map_err(|_| InvalidLicence("signature does not match"))?;
    let when = isotime::parse(&doc.expires_at).map_err(|e| match e {
        ParseError::NotIso => InvalidLicence("expires_at is not an ISO 8601 time"),
        ParseError::NoOffset => InvalidLicence("expires_at needs a timezone offset"),
    })?;
    Ok(Licence {
        doc: doc.clone(),
        expires_at: when.to_offset(UtcOffset::UTC),
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// A fixed test key pair; never the real one.
    pub fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    pub fn signed(licensee: &str, expires_at: &str) -> LicenceDoc {
        let signature = signing_key().sign(&canonical_bytes(licensee, expires_at));
        LicenceDoc {
            licensee: licensee.to_owned(),
            expires_at: expires_at.to_owned(),
            signature: STANDARD.encode(signature.to_bytes()),
        }
    }

    #[test]
    fn canonical_form_matches_python() {
        // json.dumps({"licensee": "Démo \"Bank\"", "expires_at": "x"}, sort_keys=True,
        //            separators=(",", ":"), ensure_ascii=False)
        assert_eq!(
            String::from_utf8(canonical_bytes(
                "Démo \"Bank\"\n",
                "2026-11-15T10:05:00+05:30"
            ))
            .unwrap(),
            "{\"expires_at\":\"2026-11-15T10:05:00+05:30\",\"licensee\":\"Démo \\\"Bank\\\"\\n\"}"
        );
    }

    #[test]
    fn a_signed_licence_verifies() {
        let key = signing_key().verifying_key();
        let licence = verify(
            &signed("Demo Bank", "2026-11-15T10:05:00+05:30"),
            Some(&key),
        )
        .unwrap();
        assert_eq!(
            licence.expires_at.to_string(),
            "2026-11-15 4:35:00.0 +00:00:00"
        );
    }

    #[test]
    fn edits_and_bad_files_are_refused() {
        let key = signing_key().verifying_key();
        let good = signed("Demo Bank", "2026-11-15T10:05:00+05:30");
        let edited = LicenceDoc {
            expires_at: "2027-11-15T10:05:00+05:30".into(),
            ..good.clone()
        };
        let refused = |doc: &LicenceDoc| verify(doc, Some(&key)).unwrap_err().0;
        assert_eq!(refused(&edited), "signature does not match");
        assert_eq!(
            refused(&LicenceDoc {
                signature: "%%%".into(),
                ..good.clone()
            }),
            "signature is not base64"
        );
        assert_eq!(
            refused(&LicenceDoc {
                licensee: String::new(),
                ..good.clone()
            }),
            "licensee, expires_at and signature must be non-empty strings"
        );
        assert_eq!(
            refused(&signed("x", "tomorrow")),
            "expires_at is not an ISO 8601 time"
        );
        assert_eq!(
            refused(&signed("x", "2026-11-15T10:05:00")),
            "expires_at needs a timezone offset"
        );
        assert_eq!(
            verify(&good, None).unwrap_err().0,
            "no licence public key is configured on this server"
        );
    }

    #[test]
    fn the_built_in_key_is_the_python_api_key() {
        let key = load_public_key(None).expect("public_key.pem is an Ed25519 key");
        assert_eq!(
            STANDARD.encode(key.as_bytes()),
            "CjbooCXoDT+qMKdkFkym3GcBUhrWmnPLeNRf2VOYEus="
        );
        assert_eq!(load_public_key(Some(Path::new("no/such/file.pem"))), None);
    }
}
