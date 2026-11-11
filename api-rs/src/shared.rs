//! The `shared/data` contract files, built into the binary at compile time and parsed once.
//! Identifiers (verified values, short codes) live only in those files, never in this code.

use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Items<T> {
    items: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AreaItem {
    pub pincode: String,
    pub locality: String,
    pub city: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Deserialize)]
pub struct IdItem {
    pub id: String,
}

#[derive(Deserialize)]
pub struct ValueItem {
    pub value: String,
}

#[derive(Deserialize)]
pub struct VerifiedItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

fn items<T: for<'de> Deserialize<'de>>(name: &str, text: &str) -> Vec<T> {
    serde_json::from_str::<Items<T>>(text)
        .unwrap_or_else(|e| panic!("shared/data/{name}.json does not parse: {e}"))
        .items
}

const AREAS_FILE: &str = include_str!("../../shared/data/areas.json");

pub static AREAS: LazyLock<Vec<AreaItem>> = LazyLock::new(|| items("areas", AREAS_FILE));
/// The area items exactly as written, every field included, for `/app/areas`.
pub static AREAS_RAW: LazyLock<Vec<serde_json::Value>> =
    LazyLock::new(|| items("areas", AREAS_FILE));
pub static CHANNELS: LazyLock<Vec<IdItem>> =
    LazyLock::new(|| items("channels", include_str!("../../shared/data/channels.json")));
pub static SCAM_TYPES: LazyLock<Vec<IdItem>> = LazyLock::new(|| {
    items(
        "scam_types",
        include_str!("../../shared/data/scam_types.json"),
    )
});
pub static SHORT_CODES: LazyLock<Vec<ValueItem>> = LazyLock::new(|| {
    items(
        "short_codes",
        include_str!("../../shared/data/short_codes.json"),
    )
});
pub static VERIFIED: LazyLock<Vec<VerifiedItem>> =
    LazyLock::new(|| items("verified", include_str!("../../shared/data/verified.json")));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_parses_with_the_expected_sizes() {
        assert_eq!(AREAS.len(), 25);
        assert_eq!(CHANNELS.len(), 6);
        assert_eq!(SCAM_TYPES.len(), 13);
        assert_eq!(SHORT_CODES.len(), 9);
        assert_eq!(VERIFIED.len(), 19);
    }
}
