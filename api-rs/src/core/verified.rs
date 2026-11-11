//! The PRD Appendix C verified values from `shared/data/verified.json`, normalised once.

use super::normalise::{IdentifierType, normalise};
use crate::shared;
use std::collections::HashSet;
use std::sync::LazyLock;

static VALUES: LazyLock<HashSet<(IdentifierType, String)>> = LazyLock::new(|| {
    shared::VERIFIED
        .iter()
        .map(|item| {
            let kind = IdentifierType::parse(&item.kind)
                .unwrap_or_else(|| panic!("verified.json: unknown type {}", item.kind));
            let value = normalise(kind, &item.value)
                .unwrap_or_else(|_| panic!("verified.json: {} does not normalise", item.value));
            (kind, value)
        })
        .collect()
});

pub fn is_verified(kind: IdentifierType, normalised: &str) -> bool {
    VALUES.contains(&(kind, normalised.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_value_is_verified_once_normalised() {
        assert_eq!(VALUES.len(), shared::VERIFIED.len());
        for item in shared::VERIFIED.iter() {
            let kind = IdentifierType::parse(&item.kind).unwrap();
            assert!(is_verified(kind, &normalise(kind, &item.value).unwrap()));
        }
    }

    #[test]
    fn the_type_matters() {
        let item = &shared::VERIFIED[0];
        let value = normalise(IdentifierType::parse(&item.kind).unwrap(), &item.value).unwrap();
        assert!(!is_verified(IdentifierType::Upi, &value));
    }
}
