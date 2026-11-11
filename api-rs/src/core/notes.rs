//! Note fingerprints: keyed bottom-k `MinHash` sketches of character shingles.
//!
//! A sketch keeps only the `NOTE_SKETCH_SIZE` smallest keyed hashes of the note's 5-character
//! pieces, so it can tell copies apart from people's own words but cannot be turned back into
//! text. Characters, not words, so English, Hindi and Hinglish all work the same way. Byte for
//! byte the same sketches as the Python API (keyed `BLAKE2b`, 8-byte digests, big-endian).

use super::pytext;
use super::trust_config::{NOTE_SHINGLE_CHARS, NOTE_SKETCH_SIZE, OWN_WORDS_MIN_CHARS};
use blake2::Blake2bMac;
use blake2::digest::consts::U8;
use hmac::{KeyInit, Mac};
use std::collections::BTreeSet;

/// Lower case, punctuation dropped, spacing collapsed: copies stay copies after restyling.
pub fn normalise_note(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for word in lower
        .split(|c: char| !pytext::is_alnum(c))
        .filter(|w| !w.is_empty())
    {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn hash(piece: &str, key: &[u8]) -> u64 {
    let mut mac = Blake2bMac::<U8>::new_from_slice(key).expect("note key is 32 bytes");
    mac.update(piece.as_bytes());
    u64::from_be_bytes(mac.finalize().into_bytes().into())
}

/// The fingerprint stored instead of the note. Empty for empty or very short notes.
pub fn sketch(text: &str, key: &[u8]) -> Vec<u8> {
    if pytext::strip(text).chars().count() < OWN_WORDS_MIN_CHARS {
        return Vec::new();
    }
    let clean: Vec<char> = normalise_note(text).chars().collect();
    let k = NOTE_SHINGLE_CHARS;
    let count = (clean.len() + 1).saturating_sub(k).max(1);
    let hashes: BTreeSet<u64> = (0..count)
        .map(|i| {
            clean[i..(i + k).min(clean.len())]
                .iter()
                .collect::<String>()
        })
        .map(|piece| hash(&piece, key))
        .collect();
    hashes
        .into_iter()
        .take(NOTE_SKETCH_SIZE)
        .flat_map(u64::to_be_bytes)
        .collect()
}

fn values(blob: &[u8]) -> BTreeSet<u64> {
    let (chunks, _) = blob.as_chunks::<8>();
    chunks.iter().map(|c| u64::from_be_bytes(*c)).collect()
}

/// Estimated Jaccard similarity of the two notes' shingle sets (0 when either is empty).
pub fn similarity(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let (sa, sb) = (values(a), values(b));
    let union: Vec<u64> = sa.union(&sb).copied().take(NOTE_SKETCH_SIZE).collect();
    let both = union
        .iter()
        .filter(|h| sa.contains(h) && sb.contains(h))
        .count();
    #[expect(clippy::cast_precision_loss, reason = "at most 128 values")]
    let ratio = both as f64 / union.len() as f64;
    ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::identity::Keyring;

    fn key() -> Vec<u8> {
        Keyring::new("test-key-0123456789").unwrap().note_key()
    }

    #[test]
    fn normalises_like_python() {
        assert_eq!(
            normalise_note("  Hello,   WORLD!! foo_bar "),
            "hello world foo bar"
        );
        // Python's \w excludes Hindi vowel signs, so the words split at them.
        assert_eq!(
            normalise_note("पार्सल कस्टम्स के_नाम!! Hello"),
            "\u{92a} \u{930} \u{938}\u{932} \u{915}\u{938} \u{91f}\u{92e} \u{938} \u{915} \u{928} \u{92e} hello"
        );
    }

    /// Sizes, first bytes and similarities computed with the Python API and the same key.
    #[test]
    fn sketches_match_the_python_api() {
        let a = sketch(
            "Caller said my KYC expired, asked for OTP and card number",
            &key(),
        );
        let b = sketch(
            "caller said my kyc EXPIRED asked for otp and card number!!",
            &key(),
        );
        let c = sketch(
            "पार्सल कस्टम्स के नाम पर पैसे मांगे, कॉल करने वाला पुलिस बता रहा था",
            &key(),
        );
        assert_eq!(a.len() / 8, 52);
        assert_eq!(hex::encode(&a[..16]), "0398eecf64303cf01195a10abc6867ac");
        assert_eq!(c.len() / 8, 53);
        assert_eq!(hex::encode(&c[..16]), "0292bfffd4797055076b316b09a640c5");
        assert!((similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
        assert!(similarity(&a, &c).abs() < f64::EPSILON);
    }

    #[test]
    fn short_notes_have_no_sketch() {
        assert_eq!(sketch("too short", &key()), Vec::<u8>::new());
        assert_eq!(sketch("   ", &key()), Vec::<u8>::new());
        assert!(similarity(&[], &sketch("a long enough note here", &key())).abs() < f64::EPSILON);
    }

    #[test]
    fn sketch_is_capped() {
        let long = (0..400)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" word");
        assert_eq!(sketch(&long, &key()).len(), NOTE_SKETCH_SIZE * 8);
    }
}
