//! `/map` clusters from counted reports: window, radius, top scam type, severity bands.
//! Clusters always sit at the area's Appendix B point, never at anything a reporter sent.

use super::areas::{self, Area};
use serde::Serialize;
use std::collections::HashMap;

/// Counted reports in the window at or above which an area is CRIT or HIGH; below is MED.
pub const CRIT_FROM: usize = 10;
pub const HIGH_FROM: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    #[serde(rename = "CRIT")]
    Crit,
    #[serde(rename = "HIGH")]
    High,
    #[serde(rename = "MED")]
    Med,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCount {
    pub count: usize,
    pub top_scam_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub area: &'static Area,
    pub count: usize,
    pub top_scam_type: String,
    pub severity: Severity,
}

pub fn severity(count: usize) -> Severity {
    if count >= CRIT_FROM {
        Severity::Crit
    } else if count >= HIGH_FROM {
        Severity::High
    } else {
        Severity::Med
    }
}

/// Every known area whose point is within `radius_km` of the centre's point (inclusive).
pub fn areas_within(centre: &str, radius_km: f64) -> Vec<&'static str> {
    areas::all()
        .iter()
        .filter(|a| areas::distance_km(centre, &a.pincode) <= radius_km)
        .map(|a| a.pincode.as_str())
        .collect()
}

/// One cluster per area with at least one counted report, busiest first.
pub fn clusters<S: std::hash::BuildHasher>(counts: &HashMap<String, AreaCount, S>) -> Vec<Cluster> {
    let mut found: Vec<Cluster> = counts
        .iter()
        .filter(|(_, c)| c.count > 0)
        .filter_map(|(pincode, c)| {
            Some(Cluster {
                area: areas::get(pincode)?,
                count: c.count,
                top_scam_type: c.top_scam_type.clone(),
                severity: severity(c.count),
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.area.pincode.cmp(&b.area.pincode))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_bands() {
        assert_eq!(severity(10), Severity::Crit);
        assert_eq!(severity(5), Severity::High);
        assert_eq!(severity(4), Severity::Med);
    }

    #[test]
    fn radius_includes_the_centre() {
        assert_eq!(areas_within("560102", 0.0), ["560102"]);
        assert_eq!(areas_within("560102", 5000.0).len(), areas::all().len());
    }

    #[test]
    fn clusters_busiest_first_then_by_pincode() {
        let count = |n: usize| AreaCount {
            count: n,
            top_scam_type: "kyc".into(),
        };
        let counts = HashMap::from([
            ("560103".to_owned(), count(3)),
            ("560102".to_owned(), count(3)),
            ("560001".to_owned(), count(0)),
            ("560034".to_owned(), count(12)),
        ]);
        let order: Vec<_> = clusters(&counts)
            .iter()
            .map(|c| c.area.pincode.as_str())
            .collect();
        assert_eq!(order, ["560034", "560102", "560103"]);
    }
}
