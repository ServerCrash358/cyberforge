//! Trust rules: reporter and connection clustering, copied-note merging, coordinated-burst cap,
//! weights, and report status counted / merged / suspect.
//!
//! Pure: takes every report about one identifier and returns the case's assessment. The store
//! re-runs it after each new report, so a burst found later also re-labels earlier reports.

use super::notes::similarity;
use super::trust_config::{
    BURST_INCIDENT_SPAN, BURST_MAX_WEIGHT, BURST_MIN_REPORTS, BURST_MOSTLY, BURST_WINDOW,
    NOTE_DUPLICATE_SIMILARITY, OWN_WORDS_MIN_CHARS, SAME_CONNECTION_WINDOW, WEIGHT_OWN_WORDS,
    WEIGHT_SHORT_NOTE,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Counted,
    Merged,
    Suspect,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Counted => "counted",
            Self::Merged => "merged",
            Self::Suspect => "suspect",
        }
    }
}

/// What the trust rules may see about one stored report: digests and times, no raw data.
#[derive(Debug, Clone)]
pub struct ReportFacts {
    pub id: i64,
    pub reporter: String,
    pub ip: String,
    pub reported_at: OffsetDateTime,
    pub occurred_at: OffsetDateTime,
    pub note_length: usize,
    pub sketch: Vec<u8>,
    pub first_time: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct Assessment {
    pub statuses: HashMap<i64, Status>,
    pub cluster_of: HashMap<i64, i64>,
    pub weight_of: HashMap<i64, f64>,
    pub clusters: usize,
    pub score: f64,
}

/// Python `round(x, 6)`: both round the exact binary value correctly.
fn round6(x: f64) -> f64 {
    format!("{x:.6}").parse().expect("a formatted float parses")
}

pub fn assess(reports: &[ReportFacts]) -> Assessment {
    if reports.is_empty() {
        return Assessment::default();
    }
    let mut ordered: Vec<&ReportFacts> = reports.iter().collect();
    ordered.sort_by_key(|r| (r.reported_at, r.id));
    let root = cluster(&ordered);
    let suspect = burst_members(&ordered);

    // Groups in order of their first report, like Python's insertion-ordered dict.
    let mut order: Vec<i64> = Vec::new();
    let mut members: HashMap<i64, Vec<&ReportFacts>> = HashMap::new();
    for r in &ordered {
        let group = members.entry(root[&r.id]).or_default();
        if group.is_empty() {
            order.push(root[&r.id]);
        }
        group.push(r);
    }

    let mut statuses = HashMap::new();
    let mut weight_of: HashMap<i64, f64> = ordered.iter().map(|r| (r.id, 0.0)).collect();
    let mut independent = 0;
    let mut score = 0.0;
    let mut burst_weight = 0.0;
    let mut burst_reports: Vec<i64> = Vec::new();
    for key in order {
        let group = &members[&key];
        let weight = cluster_weight(group);
        let Some(first_clean) = group.iter().find(|r| !suspect.contains(&r.id)) else {
            burst_weight += weight;
            for r in group {
                burst_reports.push(r.id);
                statuses.insert(r.id, Status::Suspect);
            }
            continue;
        };
        independent += 1;
        score += weight;
        weight_of.insert(first_clean.id, weight);
        statuses.insert(first_clean.id, Status::Counted);
        for r in group.iter().filter(|r| r.id != first_clean.id) {
            let status = if suspect.contains(&r.id) {
                Status::Suspect
            } else {
                Status::Merged
            };
            statuses.insert(r.id, status);
        }
    }

    if !burst_reports.is_empty() {
        independent += 1;
        let capped = f64::min(burst_weight, BURST_MAX_WEIGHT);
        score += capped;
        #[expect(clippy::cast_precision_loss, reason = "report counts are small")]
        let share = capped / burst_reports.len() as f64;
        for id in burst_reports {
            weight_of.insert(id, share);
        }
    }

    Assessment {
        statuses,
        cluster_of: root,
        weight_of,
        clusters: independent,
        score: round6(score),
    }
}

fn find(parent: &mut BTreeMap<i64, i64>, mut x: i64) -> i64 {
    while parent[&x] != x {
        let grand = parent[&parent[&x]];
        parent.insert(x, grand);
        x = grand;
    }
    x
}

fn union(parent: &mut BTreeMap<i64, i64>, a: i64, b: i64) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent.insert(ra.max(rb), ra.min(rb));
    }
}

/// Union-find: same reporter, same connection within a day, or near-identical notes.
fn cluster(ordered: &[&ReportFacts]) -> HashMap<i64, i64> {
    let mut parent: BTreeMap<i64, i64> = ordered.iter().map(|r| (r.id, r.id)).collect();

    let mut by_reporter: HashMap<&str, i64> = HashMap::new();
    let mut last_on_ip: HashMap<&str, &ReportFacts> = HashMap::new();
    let mut with_notes: Vec<&ReportFacts> = Vec::new();
    for r in ordered {
        if let Some(&first) = by_reporter.get(r.reporter.as_str()) {
            union(&mut parent, first, r.id);
        } else {
            by_reporter.insert(&r.reporter, r.id);
        }
        if let Some(previous) = last_on_ip.get(r.ip.as_str())
            && r.reported_at - previous.reported_at <= SAME_CONNECTION_WINDOW
        {
            union(&mut parent, previous.id, r.id);
        }
        last_on_ip.insert(&r.ip, r);
        if !r.sketch.is_empty() {
            for other in &with_notes {
                if similarity(&r.sketch, &other.sketch) >= NOTE_DUPLICATE_SIMILARITY {
                    union(&mut parent, other.id, r.id);
                }
            }
            with_notes.push(r);
        }
    }
    ordered
        .iter()
        .map(|r| (r.id, find(&mut parent, r.id)))
        .collect()
}

/// First-time reporters arriving together who look coordinated (rule 4).
fn burst_members(ordered: &[&ReportFacts]) -> HashSet<i64> {
    let fresh: Vec<&ReportFacts> = ordered.iter().copied().filter(|r| r.first_time).collect();
    let mut suspect = HashSet::new();
    let mut start = 0;
    for end in 0..fresh.len() {
        while fresh[end].reported_at - fresh[start].reported_at > BURST_WINDOW {
            start += 1;
        }
        let window = &fresh[start..=end];
        if window.len() >= BURST_MIN_REPORTS && coordinated(window) {
            suspect.extend(window.iter().map(|r| r.id));
        }
    }
    suspect
}

fn coordinated(window: &[&ReportFacts]) -> bool {
    let first = window
        .iter()
        .map(|r| r.occurred_at)
        .min()
        .expect("window is not empty");
    let last = window
        .iter()
        .map(|r| r.occurred_at)
        .max()
        .expect("window is not empty");
    if last - first <= BURST_INCIDENT_SPAN {
        return true;
    }
    let weak = window
        .iter()
        .enumerate()
        .filter(|&(i, r)| {
            r.note_length < OWN_WORDS_MIN_CHARS
                || window.iter().enumerate().any(|(j, o)| {
                    j != i && similarity(&r.sketch, &o.sketch) >= NOTE_DUPLICATE_SIMILARITY
                })
        })
        .count();
    #[expect(clippy::cast_precision_loss, reason = "window sizes are small")]
    let enough = weak as f64 >= BURST_MOSTLY * window.len() as f64;
    enough
}

fn cluster_weight(group: &[&ReportFacts]) -> f64 {
    if group.iter().any(|r| r.note_length >= OWN_WORDS_MIN_CHARS) {
        WEIGHT_OWN_WORDS
    } else {
        WEIGHT_SHORT_NOTE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::identity::Keyring;
    use crate::core::notes::sketch;
    use time::Duration;
    use time::macros::datetime;

    const T0: OffsetDateTime = datetime!(2026-11-01 06:00 UTC);

    struct Make {
        key: Vec<u8>,
        next: i64,
    }

    impl Make {
        fn new() -> Self {
            Self {
                key: Keyring::new("test-key-0123456789").unwrap().note_key(),
                next: 0,
            }
        }

        /// One report `minutes` after T0, incident at the same moment unless `incident` is set.
        fn report(
            &mut self,
            who: &str,
            ip: &str,
            minutes: usize,
            note: &str,
            first: bool,
        ) -> ReportFacts {
            self.next += 1;
            let at = T0 + Duration::minutes(i64::try_from(minutes).unwrap());
            ReportFacts {
                id: self.next,
                reporter: who.into(),
                ip: ip.into(),
                reported_at: at,
                occurred_at: at,
                note_length: note.trim().chars().count(),
                sketch: sketch(note, &self.key),
                first_time: first,
            }
        }
    }

    const NOTES: [&str; 5] = [
        "They called saying my parcel was held at customs and asked for a fee",
        "Got an SMS about an electricity bill cut tonight with a payment link",
        "Man on video call claimed to be police and said I was under digital arrest",
        "Loan app kept threatening to send morphed photos to my contacts",
        "Fake bank executive wanted my card number to stop a blocked account",
    ];

    #[test]
    fn three_independent_people_are_three_clusters() {
        let mut m = Make::new();
        let reports: Vec<_> = (0..3)
            .map(|i| {
                m.report(
                    &format!("r{i}"),
                    &format!("ip{i}"),
                    i * 600,
                    NOTES[i],
                    false,
                )
            })
            .collect();
        let a = assess(&reports);
        assert_eq!(a.clusters, 3);
        assert!((a.score - 3.0).abs() < 1e-9);
        assert!(a.statuses.values().all(|s| *s == Status::Counted));
    }

    #[test]
    fn one_person_six_times_is_one() {
        let mut m = Make::new();
        let reports: Vec<_> = (0..6)
            .map(|i| m.report("same", &format!("ip{i}"), i * 600, NOTES[0], false))
            .collect();
        let a = assess(&reports);
        assert_eq!(a.clusters, 1);
        assert_eq!(
            a.statuses
                .values()
                .filter(|s| **s == Status::Merged)
                .count(),
            5
        );
    }

    #[test]
    fn one_wifi_within_a_day_is_one() {
        let mut m = Make::new();
        let reports: Vec<_> = (0..22)
            .map(|i| m.report(&format!("acct{i}"), "wifi", i * 30, NOTES[i % 5], false))
            .collect();
        assert_eq!(assess(&reports).clusters, 1);
    }

    #[test]
    fn copied_notes_merge_across_people() {
        let mut m = Make::new();
        let reports: Vec<_> = (0..4)
            .map(|i| {
                m.report(
                    &format!("r{i}"),
                    &format!("ip{i}"),
                    i * 3000,
                    NOTES[1],
                    false,
                )
            })
            .collect();
        assert_eq!(assess(&reports).clusters, 1);
    }

    #[test]
    fn forty_friends_are_one_capped_cluster() {
        let mut m = Make::new();
        let reports: Vec<_> = (0..40)
            .map(|i| m.report(&format!("friend{i}"), &format!("ip{i}"), i, "", true))
            .collect();
        let a = assess(&reports);
        assert_eq!(a.clusters, 1);
        assert!((a.score - BURST_MAX_WEIGHT).abs() < 1e-9);
        assert!(a.statuses.values().all(|s| *s == Status::Suspect));
    }

    #[test]
    fn empty_input() {
        assert_eq!(assess(&[]), Assessment::default());
    }
}
