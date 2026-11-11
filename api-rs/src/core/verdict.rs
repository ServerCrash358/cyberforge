//! Verdict from case state: `VERIFIED_BUSINESS`, `KNOWN_SCAM`, `SUSPICIOUS` or `NO_REPORTS`.
//! A dispute (challenge) caps it at `SUSPICIOUS`.

use super::trust_config::{KNOWN_SCAM_MIN_CLUSTERS, KNOWN_SCAM_MIN_SCORE};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    KnownScam,
    Suspicious,
    NoReports,
    VerifiedBusiness,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaseFacts {
    pub report_count: usize,
    pub clusters: usize,
    pub weighted_score: f64,
    pub challenged: bool,
}

pub fn verdict(facts: CaseFacts, verified: bool) -> Verdict {
    if verified {
        return Verdict::VerifiedBusiness;
    }
    if facts.report_count == 0 {
        return Verdict::NoReports;
    }
    if !facts.challenged
        && facts.clusters >= KNOWN_SCAM_MIN_CLUSTERS
        && facts.weighted_score >= KNOWN_SCAM_MIN_SCORE
    {
        return Verdict::KnownScam;
    }
    Verdict::Suspicious
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(report_count: usize, clusters: usize, score: f64, challenged: bool) -> CaseFacts {
        CaseFacts {
            report_count,
            clusters,
            weighted_score: score,
            challenged,
        }
    }

    #[test]
    fn each_verdict() {
        assert_eq!(
            verdict(facts(9, 9, 9.0, false), true),
            Verdict::VerifiedBusiness
        );
        assert_eq!(verdict(facts(0, 0, 0.0, false), false), Verdict::NoReports);
        assert_eq!(verdict(facts(3, 3, 3.0, false), false), Verdict::KnownScam);
        assert_eq!(verdict(facts(3, 3, 3.0, true), false), Verdict::Suspicious);
        assert_eq!(verdict(facts(5, 2, 5.0, false), false), Verdict::Suspicious);
        assert_eq!(verdict(facts(3, 3, 2.8, false), false), Verdict::Suspicious);
    }

    #[test]
    fn serialises_like_the_prd() {
        assert_eq!(
            serde_json::to_string(&Verdict::KnownScam).unwrap(),
            r#""KNOWN_SCAM""#
        );
        assert_eq!(
            serde_json::to_string(&Verdict::VerifiedBusiness).unwrap(),
            r#""VERIFIED_BUSINESS""#
        );
    }
}
