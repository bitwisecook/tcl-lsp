// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Differential and property tests for the new axis-typed `VersionSet`
//! algebra (design invariant I2 and review finding B3).
//!
//! The differential half re-uses the hermetic corpus pinned from real
//! interpreters (`data/package_version_oracle.txt`, byte-identical on
//! tclsh 8.6.14 and 9.0.4): every `vsatisfies V REQ -> B` row must equal
//! `VersionSet::from_requirements([REQ]).contains(V)` — the set
//! construction and the ported `version_satisfies` may never disagree
//! about a single interpreter answer.
//!
//! The property half drives the algebra over a deterministic
//! pseudo-random grid: normalisation idempotence, and the coherence of
//! `contains` with `intersect`/`union`/`subset`.

use tcl_dialect::model::{Version, VersionAxisId, VersionSet};
use tcl_dialect::version_satisfies;

const VERSION_ORACLE: &str = include_str!("data/package_version_oracle.txt");

fn axis() -> VersionAxisId {
    VersionAxisId::package("oracle")
}

/// The data rows of the pinned corpus, tab-split.
fn rows(corpus: &str) -> impl Iterator<Item = Vec<&str>> {
    corpus
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split('\t').collect())
}

/// Every `package vsatisfies V REQ` row of the corpus, answered by set
/// construction + membership instead of the direct comparator.
#[test]
fn version_set_membership_matches_the_interpreter_over_the_pinned_corpus() {
    let mut checked = 0usize;
    let mut mismatches = Vec::new();
    for row in rows(VERSION_ORACLE) {
        let ["vsatisfies", version, requirement, want] = row.as_slice() else {
            continue;
        };
        checked += 1;
        let set = VersionSet::from_requirements(axis(), &[requirement])
            .unwrap_or_else(|err| panic!("oracle requirement `{requirement}` must parse: {err}"));
        let version = Version::parse(version)
            .unwrap_or_else(|err| panic!("oracle version `{version}` must parse: {err}"));
        let got = set.contains(&version);
        if got != (*want == "1") {
            mismatches.push(format!(
                "vsatisfies {version} {requirement}: want {want}, got {got}"
            ));
        }
    }
    assert!(
        checked >= 1000,
        "corpus shrank unexpectedly: {checked} vsatisfies rows"
    );
    assert!(
        mismatches.is_empty(),
        "{} of {checked} rows disagree with the interpreter:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The set construction and the direct `version_satisfies` port agree on
/// every corpus row too — one algebra, two query shapes.
#[test]
fn version_set_membership_matches_version_satisfies() {
    for row in rows(VERSION_ORACLE) {
        let ["vsatisfies", version, requirement, _] = row.as_slice() else {
            continue;
        };
        let set = VersionSet::from_requirements(axis(), &[requirement]).expect("oracle req");
        let parsed = Version::parse(version).expect("oracle version");
        assert_eq!(
            set.contains(&parsed),
            version_satisfies(version, requirement),
            "vsatisfies {version} {requirement}"
        );
    }
}

/// A small deterministic xorshift generator, so the property grid is
/// reproducible with no new dependencies.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let index = usize::try_from(self.next() % items.len() as u64).expect("small index");
        &items[index]
    }
}

const REQUIREMENTS: &[&str] = &[
    "1.2",
    "1.2-",
    "1.2-2.0",
    "2.0-2.0",
    "1.2a1-1.3",
    "0-",
    "8.5",
    "9",
    "8.5-9.0",
    "8.6-8.6",
    "1.3b1",
    "1.10-2.1",
    "2.0-1.0",
    "0.9-1.2",
    "3",
];

const PROBES: &[&str] = &[
    "0", "0.9", "1.1", "1.2", "1.2a0", "1.2a1", "1.2.3", "1.3", "1.3b1", "1.10", "2.0", "2.0.0",
    "2.1", "3.0", "8.4", "8.5", "8.5a0", "8.6", "8.6.14", "9.0", "9.0a0", "9.0.4", "9.1", "10.0",
];

fn random_set(rng: &mut XorShift) -> VersionSet {
    let count = usize::try_from(rng.next() % 4).expect("small count");
    let picked: Vec<&str> = (0..=count).map(|_| *rng.pick(REQUIREMENTS)).collect();
    VersionSet::from_requirements(axis(), &picked).expect("known-good requirement vocabulary")
}

/// Normalisation is idempotent: rebuilding a normalised set from its own
/// ranges reproduces it exactly.
#[test]
fn normalisation_is_idempotent_over_the_random_grid() {
    let mut rng = XorShift(0x0163_1D1A_1EC7_5EED);
    for _ in 0..500 {
        let set = random_set(&mut rng);
        let rebuilt = VersionSet::from_ranges(axis(), set.ranges().to_vec());
        assert_eq!(set, rebuilt, "{set:?}");
        // Ranges stay sorted, disjoint, and non-empty.
        for range in set.ranges() {
            assert!(!range.is_empty());
        }
        for pair in set.ranges().windows(2) {
            for probe in PROBES {
                let probe = Version::parse(probe).expect("probe");
                assert!(
                    !(pair[0].contains(&probe) && pair[1].contains(&probe)),
                    "ranges overlap at {probe}: {pair:?}"
                );
            }
        }
    }
}

/// `contains` is coherent with `intersect`, `union`, and `subset` across
/// the whole probe grid.
#[test]
fn contains_intersect_union_subset_cohere() {
    let mut rng = XorShift(0x0000_B35E_ED0F_1631);
    let probes: Vec<Version> = PROBES
        .iter()
        .map(|p| Version::parse(p).expect("probe"))
        .collect();
    for _ in 0..500 {
        let a = random_set(&mut rng);
        let b = random_set(&mut rng);
        let both = a.intersect(&b).expect("same axis");
        let either = a.union(&b).expect("same axis");
        for probe in &probes {
            assert_eq!(
                both.contains(probe),
                a.contains(probe) && b.contains(probe),
                "intersect at {probe}: {a:?} ∩ {b:?}"
            );
            assert_eq!(
                either.contains(probe),
                a.contains(probe) || b.contains(probe),
                "union at {probe}: {a:?} ∪ {b:?}"
            );
        }
        // Subset laws.
        assert!(both.subset(&a).expect("axis"));
        assert!(both.subset(&b).expect("axis"));
        assert!(a.subset(&either).expect("axis"));
        assert!(b.subset(&either).expect("axis"));
        assert!(a.subset(&a).expect("axis"));
        // subset(a, b) must agree with the probe grid's implication on
        // every probe (a ⊆ b ⇒ no probe in a \ b).
        if a.subset(&b).expect("axis") {
            for probe in &probes {
                assert!(
                    !a.contains(probe) || b.contains(probe),
                    "subset claimed but {probe} ∈ a \\ b"
                );
            }
        }
        // Union and intersection are commutative.
        assert_eq!(either, b.union(&a).expect("axis"));
        assert_eq!(both, b.intersect(&a).expect("axis"));
    }
}
