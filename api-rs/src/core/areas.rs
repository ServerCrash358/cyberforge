//! The areas of `shared/data/areas.json`: pincode lookup and great-circle distance between points.

use crate::shared::{self, AreaItem};
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub type Area = AreaItem;

const EARTH_RADIUS_KM: f64 = 6371.0;

static BY_PINCODE: LazyLock<BTreeMap<String, Area>> = LazyLock::new(|| {
    shared::AREAS
        .iter()
        .map(|a| (a.pincode.clone(), a.clone()))
        .collect()
});

/// Every area, in the order of `shared/data/areas.json`.
pub fn all() -> &'static [Area] {
    &shared::AREAS
}

pub fn get(pincode: &str) -> Option<&'static Area> {
    BY_PINCODE.get(pincode)
}

pub fn is_known(pincode: &str) -> bool {
    BY_PINCODE.contains_key(pincode)
}

/// Great-circle distance between two areas' points.
///
/// # Panics
/// When either pincode is not a known area; callers validate first.
pub fn distance_km(a: &str, b: &str) -> f64 {
    let (pa, pb) = (&BY_PINCODE[a], &BY_PINCODE[b]);
    let (lat1, lng1, lat2, lng2) = (
        pa.lat.to_radians(),
        pa.lng.to_radians(),
        pb.lat.to_radians(),
        pb.lng.to_radians(),
    );
    let h = ((lat2 - lat1) / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * ((lng2 - lng1) / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.min(1.0).sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_the_prd_areas() {
        assert!(is_known("560102") && is_known("560103"));
        assert!(!is_known("999999") && !is_known(""));
        assert_eq!(get("560102").unwrap().locality, "HSR Layout");
    }

    #[test]
    fn distance_is_symmetric_and_zero_to_itself() {
        assert!(distance_km("560102", "560102").abs() < 1e-9);
        let d = distance_km("560102", "560103");
        assert!((d - distance_km("560103", "560102")).abs() < 1e-9);
        assert!(d > 0.5 && d < 10.0, "{d}");
    }
}
