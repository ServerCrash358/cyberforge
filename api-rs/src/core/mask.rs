//! `display_value` masking until 3 distinct reporters: 98XXXXX210, r***@okaxis, sbi***.in/..., HD****.
//! Cases in `shared/vectors/mask.json`.

use super::normalise::IdentifierType;
use super::trust_config::UNMASK_MIN_REPORTERS;

/// What the app may show: the full value only once enough different people reported it.
pub fn display_value(kind: IdentifierType, normalised: &str, distinct_reporters: usize) -> String {
    if distinct_reporters >= UNMASK_MIN_REPORTERS {
        return normalised.to_owned();
    }
    match kind {
        IdentifierType::Phone => phone(normalised),
        IdentifierType::Upi => {
            let (handle, psp) = normalised.split_once('@').unwrap_or((normalised, ""));
            format!("{}***@{psp}", first(handle, 1))
        }
        IdentifierType::Url => {
            let host = normalised.strip_prefix("https://").unwrap_or(normalised);
            let host = host.split('/').next().unwrap_or_default();
            let labels: Vec<&str> = host.split('.').collect();
            let last = labels.last().copied().unwrap_or_default();
            format!("{}***.{last}/...", first(labels[0], 3))
        }
        IdentifierType::SenderId => format!("{}****", first(normalised, 2)),
    }
}

fn first(s: &str, n: usize) -> &str {
    s.char_indices().nth(n).map_or(s, |(i, _)| &s[..i])
}

fn phone(normalised: &str) -> String {
    let digits = normalised.strip_prefix("+91").unwrap_or(normalised);
    let n = digits.len();
    if n >= 8 {
        format!("{}{}{}", &digits[..2], "X".repeat(n - 5), &digits[n - 3..])
    } else {
        format!("{}{}", first(digits, 2), "X".repeat(n.saturating_sub(2)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn every_shared_vector() {
        let doc: Value =
            serde_json::from_str(include_str!("../../../shared/vectors/mask.json")).unwrap();
        for case in doc["cases"].as_array().unwrap() {
            let kind = IdentifierType::parse(case["type"].as_str().unwrap()).unwrap();
            let value = case["normalised"].as_str().unwrap();
            let n = usize::try_from(case["distinct_reporters"].as_u64().unwrap()).unwrap();
            assert_eq!(
                display_value(kind, value, n),
                case["expected"].as_str().unwrap(),
                "{kind:?} {value} {n}"
            );
        }
    }
}
