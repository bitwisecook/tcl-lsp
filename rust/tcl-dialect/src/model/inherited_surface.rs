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

//! **The enumerated half of inherit-then-override** — design **Q6**
//! (`docs/design/dialect-and-package-registry-redesign.md` §6.2, ruled
//! 2026-08-28).
//!
//! [`Ancestry`](super::family::Ancestry) lets a derived family reach its
//! ancestor's command surface without re-authoring it — the mechanism P6
//! used to collapse the jim branch's 76 hand-written core commands into
//! one edge. That is right for a [`Lineage::Fork`]: a fork *is* the
//! ancestor's source plus changes, so what the ancestor had, the fork has
//! until it says otherwise.
//!
//! It is a lie for a [`Lineage::Reimplementation`]. Jim shares no C source
//! with Tcl; it implements "a significant subset of the Tcl 8.6 command
//! set" (`jim_tcl.txt`, INTRODUCTION) — and a *subset* inherited wholesale
//! over-admits everything outside it. Measured against a built `jimsh`,
//! the inherited Tcl 8.6 surface offered a `jim` document seventeen heads
//! Jim has never had, `coroutine`, `trace`, `yield` and `yieldto` among
//! them.
//!
//! So a reimplementation edge carries a **roster**: the ancestor names the
//! descendant actually implements, each with the window on the
//! *descendant's* ladder over which it does. The roster is authored, not
//! derived — Q6 ruled it is written as `SpecTcl` (`include from tcl {…}`)
//! rather than as a second Rust catalogue — and this module is where the
//! loaded rows come to rest so that
//! [`crate::model`]'s consumers can ask one question:
//! *does this ancestor-provided name reach that descendant?*
//!
//! ## Fail-open, deliberately
//!
//! [`admits`] answers `true` for any pair with no registered roster. A
//! missing roster is a build that did not load the surface pack, and the
//! honest degradation for that is **today's behaviour** — the wholesale
//! inherited surface, over-admitting — never an empty one. A `jim`
//! document whose editor silently offered nothing at all would be a far
//! worse failure than one that offers four heads too many, and the
//! difference is invisible to the reader either way.
//!
//! ## Trust
//!
//! A roster narrows a **compiled** family's surface, so it sits at the top
//! of §6.4's lattice with the grammar declarations: only
//! [`Provenance::BuiltIn`] and [`Provenance::BundledPack`] rows register.
//! A workspace pack that could enumerate `jim`'s inherited surface could
//! delete `proc` from it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::environment::Provenance;
use super::family::{Family, Lineage};
use super::version_set::{Version, VersionSet};

/// One `(descendant ← ancestor)` roster: the ancestor-provided command
/// names the descendant implements, and the window on the descendant's
/// own ladder over which each one is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritedSurface {
    /// The reimplementing family the roster is *for* (`jim`).
    pub target: Family,
    /// The ancestor whose surface it enumerates (`tcl`).
    pub source: Family,
    /// Name → the window on [`Self::target`]'s axis. A name absent from
    /// the map is absent from the descendant.
    pub names: BTreeMap<Arc<str>, VersionSet>,
    /// Where the roster came from — see "Trust" above.
    pub provenance: Provenance,
}

impl InheritedSurface {
    /// Whether `name` reaches the descendant at `at`.
    ///
    /// `at` is the descendant axis's **point primary**, and `None` — the
    /// answer for an environment spanning the whole ladder, which is what
    /// `jim` is — takes the same permissive no-point rule
    /// `primary_admits` uses everywhere else: a name on
    /// the roster at *some* release is offered to a document that named no
    /// release.
    #[must_use]
    pub fn admits(&self, name: &str, at: Option<&Version>) -> bool {
        match self.names.get(name) {
            None => false,
            Some(window) => match at {
                Some(point) => window.contains(point),
                None => !window.is_empty(),
            },
        }
    }
}

/// Why a roster was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InheritedSurfaceError {
    /// The target family's ancestry edge is not a
    /// [`Lineage::Reimplementation`] — a fork inherits wholesale by
    /// definition, and a root family inherits nothing, so neither has a
    /// roster to carry.
    NotAReimplementation {
        /// The family the refused roster named.
        target: Family,
    },
    /// The roster's `source` is not the target's ancestor.
    NotTheAncestor {
        /// The family the refused roster named.
        target: Family,
        /// The ancestor it wrongly claimed.
        source: Family,
    },
    /// The roster came from below the trust floor (see "Trust" above).
    Untrusted {
        /// The family the refused roster named.
        target: Family,
        /// The provenance that was too low.
        provenance: Provenance,
    },
}

impl std::fmt::Display for InheritedSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAReimplementation { target } => write!(
                f,
                "`{target}` does not derive from another family by reimplementation, so it has \
                 no inherited surface to enumerate; the roster is dropped"
            ),
            Self::NotTheAncestor { target, source } => write!(
                f,
                "`{target}` does not derive from `{source}`; the roster is dropped"
            ),
            Self::Untrusted { target, provenance } => write!(
                f,
                "a roster narrowing the compiled family `{target}` must be built in or bundled, \
                 not {provenance:?}; the roster is dropped"
            ),
        }
    }
}

impl std::error::Error for InheritedSurfaceError {}

/// Whether `provenance` may narrow a compiled family's inherited surface.
#[must_use]
const fn trusted(provenance: Provenance) -> bool {
    matches!(provenance, Provenance::BuiltIn | Provenance::BundledPack)
}

/// The registered rosters, and the generation they were registered at.
#[derive(Debug, Clone, Default)]
struct RosterState {
    rosters: Vec<Arc<InheritedSurface>>,
    generation: u64,
}

static STATE: OnceLock<Mutex<Arc<RosterState>>> = OnceLock::new();

fn state_cell() -> &'static Mutex<Arc<RosterState>> {
    STATE.get_or_init(|| Mutex::new(Arc::new(RosterState::default())))
}

fn snapshot() -> Arc<RosterState> {
    Arc::clone(&state_cell().lock().expect("inherited surface lock"))
}

/// What one [`register_inherited_surfaces`] call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritedSurfaceRegistration {
    /// The generation the store moved to — the previous one when nothing
    /// changed.
    pub generation: u64,
    /// Whether the store actually changed.
    pub changed: bool,
    /// Rosters now live.
    pub rosters: usize,
    /// Rosters refused, with the rule each broke. The rest still
    /// registered.
    pub rejected: Vec<InheritedSurfaceError>,
}

/// Replace the roster store with `rosters`.
///
/// A **sync**, like [`register_dynamic_families`](super::dynamic::register_dynamic_families):
/// the caller hands over the whole set, so a roster whose pack has gone
/// retires — back to the fail-open wholesale surface, never to an empty
/// one. A roster that breaks a rule is reported and dropped; the rest
/// register. The last roster for a `(target, source)` pair wins, so a
/// bundled pack can restate a built-in one.
#[must_use]
pub fn register_inherited_surfaces(rosters: Vec<InheritedSurface>) -> InheritedSurfaceRegistration {
    let mut accepted: Vec<Arc<InheritedSurface>> = Vec::new();
    let mut rejected = Vec::new();
    for roster in rosters {
        let Some(ancestry) = roster.target.ancestry() else {
            rejected.push(InheritedSurfaceError::NotAReimplementation {
                target: roster.target,
            });
            continue;
        };
        if ancestry.lineage != Lineage::Reimplementation {
            rejected.push(InheritedSurfaceError::NotAReimplementation {
                target: roster.target,
            });
            continue;
        }
        if ancestry.parent != roster.source {
            rejected.push(InheritedSurfaceError::NotTheAncestor {
                target: roster.target,
                source: roster.source,
            });
            continue;
        }
        if !trusted(roster.provenance) {
            rejected.push(InheritedSurfaceError::Untrusted {
                target: roster.target,
                provenance: roster.provenance,
            });
            continue;
        }
        accepted.retain(|prior| prior.target != roster.target || prior.source != roster.source);
        accepted.push(Arc::new(roster));
    }
    let mut guard = state_cell().lock().expect("inherited surface lock");
    let changed = guard.rosters.len() != accepted.len()
        || guard
            .rosters
            .iter()
            .zip(&accepted)
            .any(|(live, new)| live.as_ref() != new.as_ref());
    let generation = if changed {
        guard.generation + 1
    } else {
        guard.generation
    };
    if changed {
        *guard = Arc::new(RosterState {
            rosters: accepted.clone(),
            generation,
        });
    }
    InheritedSurfaceRegistration {
        generation,
        changed,
        rosters: accepted.len(),
        rejected,
    }
}

/// The live roster for `(target ← source)`, when one is registered.
#[must_use]
pub fn roster_for(target: Family, source: Family) -> Option<Arc<InheritedSurface>> {
    snapshot()
        .rosters
        .iter()
        .find(|roster| roster.target == target && roster.source == source)
        .map(Arc::clone)
}

/// The roster store's generation — the invalidation key a cached
/// assembly holds beside the environment registry's own.
#[must_use]
pub fn inherited_surface_generation() -> u64 {
    snapshot().generation
}

/// Whether `name`, provided by `source`, reaches a `target` document at
/// `at` — **the** question the surface assembly asks.
///
/// Fail-open: `true` when no roster is registered for the pair, and
/// `true` for every pair that is not a reimplementation edge, so a fork's
/// wholesale inheritance and a build with no surface pack both keep
/// today's answer. See the module docs.
#[must_use]
pub fn admits(target: Family, source: Family, name: &str, at: Option<&Version>) -> bool {
    match roster_for(target, source) {
        None => true,
        Some(roster) => roster.admits(name, at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::VersionAxisId;

    /// The roster store is process-wide, so the tests that mutate it run
    /// under one lock and hand it back empty.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn window(spellings: &[&str]) -> VersionSet {
        VersionSet::from_requirements(VersionAxisId::core(Family::Jim), spellings)
            .expect("test window")
    }

    fn jim_roster(names: &[(&str, &[&str])]) -> InheritedSurface {
        InheritedSurface {
            target: Family::Jim,
            source: Family::Tcl,
            names: names
                .iter()
                .map(|(name, spellings)| (Arc::from(*name), window(spellings)))
                .collect(),
            provenance: Provenance::BuiltIn,
        }
    }

    fn clear() {
        let _ = register_inherited_surfaces(Vec::new());
    }

    /// The whole point: a name the roster does not carry stops reaching
    /// the descendant, while one it carries still does.
    #[test]
    fn a_roster_narrows_the_inherited_surface() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outcome = register_inherited_surfaces(vec![jim_roster(&[("proc", &["0.76-"])])]);
        assert!(outcome.changed);
        assert_eq!(outcome.rosters, 1);
        assert!(outcome.rejected.is_empty());

        assert!(admits(Family::Jim, Family::Tcl, "proc", None));
        assert!(
            !admits(Family::Jim, Family::Tcl, "coroutine", None),
            "the over-admission Q6 exists to kill"
        );
        clear();
    }

    /// Fail-open, in both of its shapes: no roster at all, and a fork
    /// edge that is not a reimplementation.
    #[test]
    fn an_unrostered_pair_inherits_wholesale() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();
        assert!(admits(Family::Jim, Family::Tcl, "coroutine", None));
        assert!(admits(Family::F5Tcl, Family::Tcl, "coroutine", None));
    }

    /// A window is read on the *descendant's* ladder — `interp` arrived
    /// at jim 0.77, so a 0.76 document does not see it while a 0.84 one
    /// does. A document naming no release keeps the permissive answer.
    #[test]
    fn a_window_is_read_on_the_descendants_ladder() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = register_inherited_surfaces(vec![jim_roster(&[("interp", &["0.77-"])])]);
        let at = |text: &str| Version::parse(text).expect("test version");

        assert!(!admits(
            Family::Jim,
            Family::Tcl,
            "interp",
            Some(&at("0.76"))
        ));
        assert!(admits(
            Family::Jim,
            Family::Tcl,
            "interp",
            Some(&at("0.84"))
        ));
        assert!(admits(Family::Jim, Family::Tcl, "interp", None));
        clear();
    }

    /// §6.4's lattice, and the two structural rules: only a
    /// reimplementation edge has a roster, only its real ancestor may be
    /// enumerated, and only a trusted tier may narrow a compiled family.
    #[test]
    fn a_roster_is_refused_off_its_edge_or_below_the_trust_floor() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fork = InheritedSurface {
            target: Family::F5Tcl,
            ..jim_roster(&[("proc", &["0.76-"])])
        };
        let root = InheritedSurface {
            target: Family::Tcl,
            ..jim_roster(&[("proc", &["0.76-"])])
        };
        let wrong_ancestor = InheritedSurface {
            source: Family::F5Irules,
            ..jim_roster(&[("proc", &["0.76-"])])
        };
        let untrusted = InheritedSurface {
            provenance: Provenance::WorkspaceUntrusted,
            ..jim_roster(&[("proc", &["0.76-"])])
        };
        let outcome = register_inherited_surfaces(vec![fork, root, wrong_ancestor, untrusted]);
        assert_eq!(outcome.rosters, 0);
        assert_eq!(outcome.rejected.len(), 4);
        assert!(matches!(
            outcome.rejected[0],
            InheritedSurfaceError::NotAReimplementation {
                target: Family::F5Tcl
            }
        ));
        assert!(matches!(
            outcome.rejected[1],
            InheritedSurfaceError::NotAReimplementation {
                target: Family::Tcl
            }
        ));
        assert!(matches!(
            outcome.rejected[2],
            InheritedSurfaceError::NotTheAncestor {
                target: Family::Jim,
                source: Family::F5Irules
            }
        ));
        assert!(matches!(
            outcome.rejected[3],
            InheritedSurfaceError::Untrusted {
                target: Family::Jim,
                provenance: Provenance::WorkspaceUntrusted
            }
        ));
        // Nothing registered, so the pair is still fail-open.
        assert!(admits(Family::Jim, Family::Tcl, "coroutine", None));
        clear();
    }

    /// A sync retires what the new set omits, and re-registering the same
    /// set does not move the generation (the reload answer).
    #[test]
    fn registration_is_a_sync_with_a_stable_generation() {
        let _serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();
        let first = register_inherited_surfaces(vec![jim_roster(&[("proc", &["0.76-"])])]);
        let again = register_inherited_surfaces(vec![jim_roster(&[("proc", &["0.76-"])])]);
        assert!(!again.changed);
        assert_eq!(again.generation, first.generation);

        let retired = register_inherited_surfaces(Vec::new());
        assert!(retired.changed);
        assert_eq!(retired.rosters, 0);
        assert!(admits(Family::Jim, Family::Tcl, "coroutine", None));
        clear();
    }
}
