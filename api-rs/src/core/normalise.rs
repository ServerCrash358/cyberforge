//! Turns phone, UPI, URL and `sender_id` values into their single standard form.
//! Rules in ARCHITECTURE.md "Normalisation rules"; cases in `shared/vectors/normalise.json`.

use super::pytext;
use crate::shared;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierType {
    Phone,
    Upi,
    Url,
    SenderId,
}

impl IdentifierType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Upi => "upi",
            Self::Url => "url",
            Self::SenderId => "sender_id",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "phone" => Some(Self::Phone),
            "upi" => Some(Self::Upi),
            "url" => Some(Self::Url),
            "sender_id" => Some(Self::SenderId),
            _ => None,
        }
    }
}

/// The value cannot be a valid identifier of its type; the API replies 400.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidIdentifier(pub IdentifierType);

static PHONE_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9+\-.() ]+$").unwrap());
static UPI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9._-]{2,256}@[a-z][a-z0-9]{1,63}$").unwrap());
static HOST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,63}$").unwrap());
static SENDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Z]{2}-)?([A-Z]{6})(?:-[A-Z])?$").unwrap());
static SHORT_CODES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    shared::SHORT_CODES
        .iter()
        .map(|c| c.value.clone())
        .collect()
});

const TOLL_FREE: [&str; 2] = ["1800", "1860"];

/// The standard form of `raw`.
///
/// # Errors
/// A value that cannot be an identifier of this type.
pub fn normalise(kind: IdentifierType, raw: &str) -> Result<String, InvalidIdentifier> {
    let found = match kind {
        IdentifierType::Phone => phone(raw),
        IdentifierType::Upi => upi(raw),
        IdentifierType::Url => url(raw),
        IdentifierType::SenderId => sender(raw),
    };
    found.ok_or(InvalidIdentifier(kind))
}

fn phone(raw: &str) -> Option<String> {
    let text = pytext::strip(raw);
    if text.is_empty() || !PHONE_CHARS.is_match(text) {
        return None;
    }
    let bare: String = text.chars().filter(|c| !matches!(c, '(' | ')')).collect();
    let bare = bare.trim_matches(' ');
    let has_plus = bare.starts_with('+');
    if bare.get(1..).is_some_and(|rest| rest.contains('+')) {
        return None;
    }
    let digits: String = bare.chars().filter(char::is_ascii_digit).collect();

    if has_plus {
        return digits.strip_prefix("91").and_then(indian);
    }
    if SHORT_CODES.contains(&digits) {
        return Some(digits);
    }
    if TOLL_FREE.iter().any(|p| digits.starts_with(p)) {
        return (4..8).contains(&(digits.len() - 4)).then_some(digits);
    }
    if let Some(national) = digits.strip_prefix("0091") {
        return indian(national);
    }
    if digits.len() == 12 && digits.starts_with("91") {
        return indian(&digits[2..]);
    }
    if digits.len() == 11 && digits.starts_with('0') {
        return indian(&digits[1..]);
    }
    indian(&digits)
}

fn indian(national: &str) -> Option<String> {
    let ok = national.len() == 10 && national.starts_with(['2', '3', '4', '5', '6', '7', '8', '9']);
    ok.then(|| format!("+91{national}"))
}

fn upi(raw: &str) -> Option<String> {
    let value = pytext::split(raw).collect::<String>().to_lowercase();
    UPI.is_match(&value).then_some(value)
}

fn url(raw: &str) -> Option<String> {
    let text = pytext::strip(raw);
    if text.is_empty() || text.chars().any(pytext::is_space) {
        return None;
    }
    let rest = match text.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = scheme.to_lowercase();
            if scheme != "http" && scheme != "https" {
                return None;
            }
            rest
        }
        None => text,
    };
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = match rest.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (rest, String::new()),
    };
    let mut host = host.to_lowercase();
    if let Some(bare) = host.strip_prefix("www.")
        && bare.contains('.')
    {
        host = bare.to_owned();
    }
    if !HOST.is_match(&host) {
        return None;
    }
    Some(format!("https://{host}{}", path.trim_end_matches('/')))
}

fn sender(raw: &str) -> Option<String> {
    let upper = pytext::strip(raw).to_uppercase();
    SENDER.captures(&upper).map(|c| c[1].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn every_shared_vector() {
        let doc: Value =
            serde_json::from_str(include_str!("../../../shared/vectors/normalise.json")).unwrap();
        let cases = doc["cases"].as_array().unwrap();
        assert!(cases.len() > 100);
        for case in cases {
            let kind = IdentifierType::parse(case["type"].as_str().unwrap()).unwrap();
            let input = case["input"].as_str().unwrap();
            let got = normalise(kind, input).ok();
            assert_eq!(
                got.as_deref(),
                case["expected"].as_str(),
                "{kind:?} {input:?}"
            );
        }
    }

    #[test]
    fn type_names_round_trip() {
        for kind in [
            IdentifierType::Phone,
            IdentifierType::Upi,
            IdentifierType::Url,
            IdentifierType::SenderId,
        ] {
            assert_eq!(IdentifierType::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(IdentifierType::parse("email"), None);
    }
}
