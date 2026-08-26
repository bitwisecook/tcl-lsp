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

//! The **semantic view**'s binding vocabulary (redesign §4.2) — types and
//! contract only in P1-E; realm integration is P1a.
//!
//! ## The consumer contract (invariants I3–I5)
//!
//! - **I3 — the type split.** Semantic passes (diagnostics that assert,
//!   code actions that edit, taint, lowering, codegen) consume
//!   [`BindingKnowledge`] at a program point; assistance surfaces consume
//!   [`crate::model::context::ContextQueries`]. The two have different
//!   names and different types so a semantic pass cannot call the
//!   assistance shortcut by accident.
//! - **I4 — no hook before proof.** No taint, side-effect, lowering, or
//!   codegen hook is selected before its binding is proved
//!   [`BindingKnowledge::Must`]. Ambiguity ([`BindingKnowledge::May`])
//!   takes the conservative union of effects or abstains; it never picks
//!   a candidate by catalogue order or provider specificity — authoring
//!   precedence is not binding resolution (review B4).
//! - **I5 — permutations widen.** Load-order permutations that change the
//!   real binding (two packages exporting one name; `namespace import`
//!   with and without `-force`) must change — or widen — the resolved
//!   answer, never silently keep a stale `Must`.
//!
//! Package state is **per interpreter and temporal** (review B2): the
//! package table lives on the interpreter, `ifneeded`/`unknown` handlers
//! run arbitrary scripts, a child interpreter inherits nothing, and a
//! provided version proves nothing about the live command table — which is
//! why [`PackageState`] and [`PackageTransition`] are realm vocabulary
//! here rather than document-global floors.
//!
//! [`PackageTransition`] is deliberately a **parallel type**, not new
//! variants on [`crate::state_transition::StateTransition`]: that enum is
//! matched exhaustively by consumers across the workspace (analyser,
//! compiler, LSP), so growing it is a breaking change those crates must
//! opt into. **P1a integration point**: `RealmState` composes the
//! existing command-binding lattice, `InterpreterTransition`, and this
//! package family; at that point either `StateTransition` gains a
//! `Package(PackageTransition)` variant in a coordinated change, or the
//! realm layer keeps consuming the two families side by side.

use std::sync::Arc;

use rustc_hash::FxHashMap;
use tcl_dialect::model::Version;

use crate::spec::CommandSpec;
use crate::state_transition::TransitionSubject;

/// The identity of one resolved spec — what a proved binding points at.
///
/// Identity is the spec allocation itself: compiled specs are interned
/// `&'static` values, so two keys are equal exactly when they name the
/// same registration. The P2 generation work re-keys dynamic pack specs
/// by `(generation, registration)` when non-`'static` specs join.
#[derive(Clone, Copy)]
pub struct SpecKey(&'static CommandSpec);

impl SpecKey {
    /// The key for `spec`.
    #[must_use]
    pub fn new(spec: &'static CommandSpec) -> Self {
        Self(spec)
    }

    /// The spec this key names.
    #[must_use]
    pub fn spec(self) -> &'static CommandSpec {
        self.0
    }

    /// The spec's command name.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name
    }
}

impl PartialEq for SpecKey {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for SpecKey {}

impl std::hash::Hash for SpecKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.0).hash(state);
    }
}

impl std::fmt::Debug for SpecKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SpecKey").field(&self.0.name).finish()
    }
}

/// What a realm proves about one command name at one program point
/// (§4.2) — the single `exists` oracle of centralisation contract R-c.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKnowledge {
    /// Proved absent: nothing provides the name here (the closed-world
    /// guarantee iRules derives rather than assumes).
    Absent,
    /// Proved: exactly this binding, here. The only state that licenses
    /// hook selection (I4).
    Must(SpecKey),
    /// Known candidates, but order/branch not proved (two providers of
    /// one name, a conditional require). Consumers take the union of
    /// effects or abstain — never a pick by catalogue precedence (I4).
    May(Arc<[SpecKey]>),
    /// Nothing provable: a dynamic loader, an unknown interp target, a
    /// widened domain.
    Unknown,
}

impl BindingKnowledge {
    /// Whether this knowledge licenses semantic hook selection (I4): only
    /// a proved [`Self::Must`] does.
    #[must_use]
    pub const fn is_proved(&self) -> bool {
        matches!(self, Self::Must(_))
    }

    /// The candidate set a conservative consumer may union over: the
    /// proved binding, the `May` candidates, or nothing for `Absent` /
    /// `Unknown` (an unknown binding has no enumerable candidates —
    /// consumers abstain).
    #[must_use]
    pub fn candidates(&self) -> &[SpecKey] {
        match self {
            Self::Must(key) => std::slice::from_ref(key),
            Self::May(keys) => keys,
            Self::Absent | Self::Unknown => &[],
        }
    }

    /// The conservative join of two flow edges' knowledge (I5): equal
    /// knowledge survives, differing proofs widen to the candidate union,
    /// and absence meeting a binding widens to [`Self::Unknown`] — `May`
    /// enumerates candidates, not the possibility of absence, so absence
    /// can never hide inside it.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (left, right) if left == right => left.clone(),
            // Unknown is absorbing; unequal Absent widens too, because
            // `May` cannot enumerate the possibility of absence.
            (Self::Unknown | Self::Absent, _) | (_, Self::Unknown | Self::Absent) => Self::Unknown,
            (left, right) => {
                let mut keys: Vec<SpecKey> = Vec::new();
                for key in left.candidates().iter().chain(right.candidates()) {
                    if !keys.contains(key) {
                        keys.push(*key);
                    }
                }
                Self::May(keys.into())
            }
        }
    }
}

/// One package's state in one interpreter's realm (§4.2's
/// `PackageStateMap` values), temporal by design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageState {
    /// Nothing known — the default for every package no transition has
    /// touched.
    Unknown,
    /// Known loadable here (an `ifneeded` script is registered, or the
    /// context's world ships it) but not yet provided.
    Available,
    /// A `package require` is mid-flight: its `ifneeded`/`unknown`
    /// scripts are running, so the realm is observably between states
    /// (re-entrant requires see this).
    Loading,
    /// `package provide` recorded this version. Deliberately **not** a
    /// proof about the live command table (review B2): commands may have
    /// been renamed away while the provision stands.
    Provided(Version),
}

/// Per-interpreter package states, defaulting to
/// [`PackageState::Unknown`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageStateMap {
    entries: FxHashMap<Arc<str>, PackageState>,
}

impl PackageStateMap {
    /// An empty map — every package [`PackageState::Unknown`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The state of `package`.
    #[must_use]
    pub fn state(&self, package: &str) -> &PackageState {
        self.entries.get(package).unwrap_or(&PackageState::Unknown)
    }

    /// Record `state` for `package`.
    pub fn set(&mut self, package: &str, state: PackageState) {
        self.entries.insert(Arc::from(package), state);
    }

    /// The recorded entries (packages some transition touched), in
    /// arbitrary order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &PackageState)> {
        self.entries
            .iter()
            .map(|(package, state)| (package.as_ref(), state))
    }
}

/// The package-domain transition family (§4.2) — the state changes the
/// realm layer applies to a [`PackageStateMap`]. Shaped like the
/// [`crate::state_transition`] families: literal operands keep their
/// value, dynamic operands stay [`TransitionSubject::Unknown`] and widen
/// the affected domain at the consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageTransition {
    /// `package provide NAME ?VERSION?` — records a provision; proves
    /// nothing about commands (B2).
    Provide {
        /// The provided package name.
        package: TransitionSubject,
        /// The provided version, when the call states one.
        version: Option<TransitionSubject>,
    },
    /// `package require ?-exact? NAME ?REQUIREMENT…?` — may run
    /// `ifneeded`/`unknown` scripts and re-enter the realm.
    Require {
        /// The required package name.
        package: TransitionSubject,
        /// The requirement words, in call order.
        requirements: Vec<TransitionSubject>,
        /// Whether `-exact` pinned the requirement.
        exact: bool,
    },
    /// `package ifneeded NAME VERSION ?SCRIPT?` — registers (or queries)
    /// a load script, making the package [`PackageState::Available`].
    Ifneeded {
        /// The package name.
        package: TransitionSubject,
        /// The version the script provides.
        version: TransitionSubject,
        /// Whether a script was supplied (the registering form) rather
        /// than queried.
        script_provided: bool,
    },
    /// `package forget ?NAME…?` — drops provisions and ifneeded scripts.
    Forget {
        /// The forgotten package names.
        packages: Vec<TransitionSubject>,
    },
    /// `package unknown ?HANDLER?` — replaces the interpreter's fallback
    /// loader; an unknown handler widens every later require.
    UnknownHandler {
        /// The new handler command prefix, when set.
        handler: Option<TransitionSubject>,
    },
    /// A `source`/`load` that can define commands and provide packages
    /// outside any `package` bookkeeping — the widening ingress of the
    /// package domain.
    SourceLoad {
        /// The sourced or loaded path.
        path: TransitionSubject,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &'static str) -> SpecKey {
        // Two distinct static specs give two distinct identities.
        fn leak(name: &'static str) -> &'static CommandSpec {
            Box::leak(Box::new(CommandSpec {
                name,
                ..CommandSpec::DEFAULT
            }))
        }
        SpecKey::new(leak(name))
    }

    #[test]
    fn spec_keys_are_allocation_identities() {
        let a = key("alpha");
        let b = key("alpha");
        assert_ne!(a, b, "same name, different registration");
        assert_eq!(a, a);
        assert_eq!(a.name(), "alpha");
        assert!(std::ptr::eq(a.spec(), a.spec()));
    }

    #[test]
    fn only_must_licenses_hooks() {
        let must = BindingKnowledge::Must(key("proved"));
        assert!(must.is_proved());
        assert_eq!(must.candidates().len(), 1);
        for unproved in [
            BindingKnowledge::Absent,
            BindingKnowledge::Unknown,
            BindingKnowledge::May(vec![key("a"), key("b")].into()),
        ] {
            assert!(!unproved.is_proved(), "{unproved:?}");
        }
        assert!(BindingKnowledge::Unknown.candidates().is_empty());
        assert!(BindingKnowledge::Absent.candidates().is_empty());
    }

    #[test]
    fn joins_widen_and_never_narrow() {
        let a = key("a");
        let b = key("b");
        let must_a = BindingKnowledge::Must(a);
        let must_b = BindingKnowledge::Must(b);
        // Idempotent.
        assert_eq!(must_a.join(&must_a), must_a);
        // Differing proofs widen to the candidate union, symmetrically.
        let joined = must_a.join(&must_b);
        assert_eq!(joined, BindingKnowledge::May(vec![a, b].into()));
        assert_eq!(
            must_b.join(&must_a),
            BindingKnowledge::May(vec![b, a].into())
        );
        assert!(!joined.is_proved());
        // May absorbs an already-listed candidate without duplicating.
        assert_eq!(joined.join(&must_a), joined);
        // Absence meeting a binding cannot hide inside May.
        assert_eq!(
            must_a.join(&BindingKnowledge::Absent),
            BindingKnowledge::Unknown
        );
        // Unknown is absorbing.
        assert_eq!(
            joined.join(&BindingKnowledge::Unknown),
            BindingKnowledge::Unknown
        );
        // Equal non-Must knowledge survives.
        assert_eq!(
            BindingKnowledge::Absent.join(&BindingKnowledge::Absent),
            BindingKnowledge::Absent
        );
    }

    #[test]
    fn package_states_default_unknown_and_are_temporal() {
        let mut map = PackageStateMap::new();
        assert_eq!(map.state("http"), &PackageState::Unknown);
        map.set("http", PackageState::Available);
        map.set("http", PackageState::Loading);
        map.set(
            "http",
            PackageState::Provided(Version::parse("2.10").expect("version")),
        );
        assert_eq!(
            map.state("http"),
            &PackageState::Provided(Version::parse("2.10.0").expect("version")),
            "provided versions compare by the comparator, not spelling"
        );
        assert_eq!(map.entries().count(), 1);
        assert_eq!(map.state("tls"), &PackageState::Unknown);
    }

    #[test]
    fn package_transitions_keep_dynamic_operands_opaque() {
        let dynamic = TransitionSubject::Unknown {
            argument_index: 1,
            word_kind: crate::invocation_words::InvocationWordKind::Dynamic,
        };
        let require = PackageTransition::Require {
            package: dynamic.clone(),
            requirements: vec![TransitionSubject::Literal("8.5".to_owned())],
            exact: false,
        };
        let PackageTransition::Require { package, .. } = &require else {
            unreachable!("just built");
        };
        assert_eq!(package.literal(), None, "dynamic operands stay opaque");
        let provide = PackageTransition::Provide {
            package: TransitionSubject::Literal("demo".to_owned()),
            version: Some(TransitionSubject::Literal("1.0".to_owned())),
        };
        assert_ne!(provide, require);
    }
}
