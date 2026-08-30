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

//! **Pack-declared dialects as runtime family data** — the P3 rider of
//! the redesign's §6.2 `dialect` block, and the first step of Q1's
//! endgame.
//!
//! A `dialect NAME { … }` block sets values for axes *Rust* defines, so
//! what a pack declares is not a new grammar mechanism but a new point in
//! the space the compiled [`LexerGrammar`] already spans. That makes the
//! conversion total and checkable: the block's axis values project onto a
//! `LexerGrammar` (the loader's `PackDialect::to_grammar`), the §2
//! classification gate has already refused a projection equal to a
//! compiled family release, and what lands here is the **validated
//! runtime representation** of what is left — a [`DynamicFamily`] with a
//! release ladder, a grammar per release, a trust class, and a namespaced
//! id.
//!
//! # What this is, and what it is not
//!
//! It is *not* a new [`Family`] variant. [`Family`] is a closed enum whose
//! [`grammar`](crate::model::grammar) is a `const fn` over ladder
//! ordinals, and [`Release`](crate::model::Release) values only exist on
//! those ladders — by
//! construction, since the compiled catalogue's soundness rests on both
//! being exhaustive. A pack-declared family therefore cannot be named by
//! a [`CoreProfileSelector`](crate::model::CoreProfileSelector), and an
//! [`EnvironmentDefinition`](crate::model::EnvironmentDefinition) that
//! rides one carries `core: None` plus a **binding** registered here.
//!
//! The boundary that stops it going further is the lexer's, not this
//! module's: `tcl_lexer::LexerConfig` is built from a
//! `&'static DialectProfile`'s `grammar` field, and the ingress hands
//! consumers `&'static DialectProfile` values from a compiled table. A
//! dynamic family has a `LexerGrammar` — [`dynamic_core_grammar`] returns
//! it — and nothing on the analysis path can yet be given one that is not
//! a compiled profile's. Closing that is the `DialectProfile` re-type
//! (ledger C1), not a change here.
//!
//! # Trust (§6.4)
//!
//! Grammar declarations sit at the top of the trust lattice:
//!
//! - **Compiled family names are reserved.** `tcl`, `f5-tcl`,
//!   `f5-irules`, `jim` — see [`reserved_family_name`] — may not be
//!   declared, at any tier.
//! - **Third-party dialects are namespaced**, pack-name-prefixed
//!   (`spicegentcl/ngspice`), exactly as §3.3 namespaces third-party
//!   environment ids. Namespacing is the caller's to apply; this module
//!   refuses an id that would collide with a compiled family name and
//!   refuses two registrations of one id.
//! - Every family carries the declaring tier's [`Provenance`], so a
//!   consumer can say who declared the grammar it is reading.

use std::sync::{Arc, Mutex, OnceLock};

use crate::LexerGrammar;
use crate::model::{BuildProfileId, Family, Provenance};

/// The namespaced, stable id of a pack-declared family
/// (`spicegentcl/ngspice`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DynamicFamilyId(Arc<str>);

impl DynamicFamilyId {
    /// An id from its canonical spelling.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(Arc::from(id))
    }

    /// The canonical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DynamicFamilyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One release on a pack-declared family's ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRelease {
    /// The release's spelling on the declared ladder, as the pack wrote
    /// it. Deliberately a string: it is not a [`crate::model::Release`],
    /// because that type's values exist only on compiled ladders.
    pub name: Arc<str>,
    /// The build profile the ladder row names.
    pub build: BuildProfileId,
    /// The lexical grammar of this release.
    pub grammar: LexerGrammar,
}

/// A pack-declared language family, converted to runtime data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicFamily {
    /// The namespaced id.
    pub id: DynamicFamilyId,
    /// The human-facing name.
    pub display_name: Arc<str>,
    /// The release ladder, oldest first, never empty.
    pub releases: Vec<DynamicRelease>,
    /// The declaring tier's trust class.
    pub provenance: Provenance,
}

impl DynamicFamily {
    /// The named release on this ladder.
    #[must_use]
    pub fn release(&self, name: &str) -> Option<&DynamicRelease> {
        self.releases.iter().find(|release| &*release.name == name)
    }

    /// The ladder's newest release — the default when an environment's
    /// `core` row names the family without a release.
    #[must_use]
    pub fn newest(&self) -> &DynamicRelease {
        self.releases
            .last()
            .expect("a dynamic family always has at least one release")
    }
}

/// One environment's binding to a pack-declared core.
///
/// The stand-in for the
/// [`CoreProfileSelector`](crate::model::CoreProfileSelector) an
/// environment cannot hold for a family that is not a [`Family`]
/// variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicCore {
    /// The environment's canonical id.
    pub environment: String,
    /// The family it rides.
    pub family: DynamicFamilyId,
    /// The release on that family's ladder.
    pub release: Arc<str>,
}

/// Why a pack-declared family did not register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicFamilyError {
    /// The id claims a compiled family name.
    Reserved {
        /// The reserved spelling that was claimed.
        name: String,
        /// The claiming id.
        claimed_by: String,
        /// The claiming family's trust class.
        provenance: Provenance,
    },
    /// Two registered families claim one id.
    DuplicateId(String),
    /// A ladder with no releases is not a ladder.
    EmptyLadder(String),
    /// A core binding names a family nothing declares.
    UnknownFamily {
        /// The environment whose binding could not resolve.
        environment: String,
        /// The unresolvable family id.
        family: String,
    },
    /// A core binding names a release the family's ladder does not carry.
    UnknownRelease {
        /// The environment whose binding could not resolve.
        environment: String,
        /// The family the binding names.
        family: String,
        /// The unresolvable release spelling.
        release: String,
    },
}

impl std::fmt::Display for DynamicFamilyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reserved {
                name,
                claimed_by,
                provenance,
            } => write!(
                f,
                "dialect `{claimed_by}` claims `{name}`, a compiled family name, from the \
                 {provenance:?} tier; compiled family names are reserved (design §6.2/§6.4)"
            ),
            Self::DuplicateId(id) => {
                write!(f, "two pack-declared dialects claim the id `{id}`")
            }
            Self::EmptyLadder(id) => write!(
                f,
                "dialect `{id}` declares no `release` row, so it has no ladder to place \
                 a core on"
            ),
            Self::UnknownFamily {
                environment,
                family,
            } => write!(
                f,
                "environment `{environment}` names core family `{family}`, which no \
                 compiled family and no loaded pack declares"
            ),
            Self::UnknownRelease {
                environment,
                family,
                release,
            } => write!(
                f,
                "environment `{environment}` names core `{family} {release}`, which is \
                 not a release on that dialect's ladder"
            ),
        }
    }
}

impl std::error::Error for DynamicFamilyError {}

/// The compiled family name `name` claims, when it claims one.
///
/// Every compiled [`Family`] name is reserved at every tier: a pack that
/// could redeclare `tcl`'s grammar could silently change how every Tcl
/// document in the workspace lexes.
#[must_use]
pub fn reserved_family_name(name: &str) -> Option<&'static str> {
    Family::ALL
        .iter()
        .map(|family| family.name())
        .find(|compiled| *compiled == name)
}

/// The registered families and core bindings, plus the generation they
/// were registered at.
#[derive(Debug, Clone, Default)]
struct DynamicState {
    families: Vec<Arc<DynamicFamily>>,
    cores: Vec<DynamicCore>,
    generation: u64,
}

static STATE: OnceLock<Mutex<Arc<DynamicState>>> = OnceLock::new();

fn state_cell() -> &'static Mutex<Arc<DynamicState>> {
    STATE.get_or_init(|| Mutex::new(Arc::new(DynamicState::default())))
}

fn snapshot() -> Arc<DynamicState> {
    Arc::clone(&state_cell().lock().expect("dynamic family lock"))
}

/// What one [`register_dynamic_families`] call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRegistration {
    /// The generation the store moved to — the previous one when
    /// nothing changed.
    pub generation: u64,
    /// Whether the store actually changed.
    pub changed: bool,
    /// Families now live.
    pub families: usize,
    /// Environment core bindings now live.
    pub cores: usize,
    /// Families and bindings refused, with the rule each broke. The rest
    /// still registered.
    pub rejected: Vec<DynamicFamilyError>,
}

/// Replace the pack-declared family store with `families` and their
/// environment `cores`.
///
/// A **sync**, like the environment registry's source channel: the caller
/// hands over the whole set, so a dialect whose pack has left the
/// workspace retires. A family or binding that breaks a rule is reported
/// and dropped; the rest register.
#[must_use]
pub fn register_dynamic_families(
    families: Vec<DynamicFamily>,
    cores: Vec<DynamicCore>,
) -> DynamicRegistration {
    let mut accepted: Vec<Arc<DynamicFamily>> = Vec::new();
    let mut rejected = Vec::new();
    for family in families {
        if let Some(name) = reserved_family_name(family.id.as_str()) {
            rejected.push(DynamicFamilyError::Reserved {
                name: name.to_owned(),
                claimed_by: family.id.as_str().to_owned(),
                provenance: family.provenance,
            });
            continue;
        }
        if family.releases.is_empty() {
            rejected.push(DynamicFamilyError::EmptyLadder(
                family.id.as_str().to_owned(),
            ));
            continue;
        }
        if accepted.iter().any(|prior| prior.id == family.id) {
            rejected.push(DynamicFamilyError::DuplicateId(
                family.id.as_str().to_owned(),
            ));
            continue;
        }
        accepted.push(Arc::new(family));
    }

    let mut bound = Vec::new();
    for core in cores {
        let Some(family) = accepted.iter().find(|family| family.id == core.family) else {
            rejected.push(DynamicFamilyError::UnknownFamily {
                environment: core.environment.clone(),
                family: core.family.as_str().to_owned(),
            });
            continue;
        };
        if family.release(&core.release).is_none() {
            rejected.push(DynamicFamilyError::UnknownRelease {
                environment: core.environment.clone(),
                family: core.family.as_str().to_owned(),
                release: core.release.to_string(),
            });
            continue;
        }
        bound.push(core);
    }

    let mut guard = state_cell().lock().expect("dynamic family lock");
    let changed = guard.families != accepted || guard.cores != bound;
    let generation = if changed {
        guard.generation + 1
    } else {
        guard.generation
    };
    let families = accepted.len();
    let cores = bound.len();
    if changed {
        *guard = Arc::new(DynamicState {
            families: accepted,
            cores: bound,
            generation,
        });
    }
    DynamicRegistration {
        generation,
        changed,
        families,
        cores,
        rejected,
    }
}

/// Every pack-declared family currently live.
#[must_use]
pub fn dynamic_families() -> Vec<Arc<DynamicFamily>> {
    snapshot().families.clone()
}

/// The pack-declared family `name` resolves to — by canonical
/// (namespaced) id, or by the bare declared name when exactly one loaded
/// pack claims it.
#[must_use]
pub fn resolve_dynamic_family(name: &str) -> Option<Arc<DynamicFamily>> {
    let state = snapshot();
    if let Some(exact) = state
        .families
        .iter()
        .find(|family| family.id.as_str() == name)
    {
        return Some(Arc::clone(exact));
    }
    let mut bare = state
        .families
        .iter()
        .filter(|family| family.id.as_str().rsplit('/').next() == Some(name));
    let first = bare.next()?;
    if bare.next().is_some() {
        // Ambiguous across packs: the namespaced id is the only honest
        // answer, so refuse rather than pick one.
        return None;
    }
    Some(Arc::clone(first))
}

/// The pack-declared core `environment` rides, when it rides one.
#[must_use]
pub fn dynamic_core_for(environment: &str) -> Option<(Arc<DynamicFamily>, DynamicRelease)> {
    let state = snapshot();
    let core = state
        .cores
        .iter()
        .find(|core| core.environment == environment)?;
    let family = state
        .families
        .iter()
        .find(|family| family.id == core.family)?;
    let release = family.release(&core.release)?.clone();
    Some((Arc::clone(family), release))
}

/// The lexical grammar of the pack-declared core `environment` rides.
///
/// The answer the analysis path would need in order to lex a document in
/// a pack-declared dialect. Nothing on that path consumes it yet — see
/// this module's header for exactly where the lexer boundary sits — so
/// this is the seam a consumer will read from, and the proof the
/// conversion produces a real grammar rather than a description of one.
#[must_use]
pub fn dynamic_core_grammar(environment: &str) -> Option<LexerGrammar> {
    dynamic_core_for(environment).map(|(_, release)| release.grammar)
}

/// The store's generation — bumped by every registration that changed
/// it, so a consumer can cache against it.
#[must_use]
pub fn dynamic_generation() -> u64 {
    snapshot().generation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::grammar;

    /// The whole store is one process-global, so the tests that write to
    /// it take this first.
    static WRITING: Mutex<()> = Mutex::new(());

    fn family(id: &str, provenance: Provenance) -> DynamicFamily {
        DynamicFamily {
            id: DynamicFamilyId::new(id),
            display_name: Arc::from(id),
            releases: vec![DynamicRelease {
                name: Arc::from("1.0"),
                build: BuildProfileId::Canonical,
                grammar: LexerGrammar {
                    expand_syntax: false,
                    ..grammar(Family::Tcl, crate::model::Release::TCL_9_0)
                },
            }],
            provenance,
        }
    }

    #[test]
    fn a_registered_family_resolves_by_id_and_bare_name_and_retires() {
        let _writing = WRITING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outcome = register_dynamic_families(
            vec![family("probepack/picol2", Provenance::User)],
            vec![DynamicCore {
                environment: "picol-shell".to_owned(),
                family: DynamicFamilyId::new("probepack/picol2"),
                release: Arc::from("1.0"),
            }],
        );
        assert!(outcome.changed);
        assert_eq!(outcome.families, 1);
        assert_eq!(outcome.cores, 1);
        assert!(outcome.rejected.is_empty());
        assert!(resolve_dynamic_family("probepack/picol2").is_some());
        assert!(resolve_dynamic_family("picol2").is_some());
        let grammar = dynamic_core_grammar("picol-shell").expect("the bound grammar");
        assert!(!grammar.expand_syntax);

        let retired = register_dynamic_families(Vec::new(), Vec::new());
        assert!(retired.changed);
        assert!(resolve_dynamic_family("probepack/picol2").is_none());
        assert!(dynamic_core_grammar("picol-shell").is_none());
    }

    #[test]
    fn a_compiled_family_name_is_reserved_and_a_dangling_core_is_refused() {
        let _writing = WRITING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outcome = register_dynamic_families(
            vec![family("tcl", Provenance::BundledPack)],
            vec![DynamicCore {
                environment: "nowhere".to_owned(),
                family: DynamicFamilyId::new("probepack/absent"),
                release: Arc::from("1.0"),
            }],
        );
        assert_eq!(outcome.families, 0);
        assert!(outcome.rejected.iter().any(
            |error| matches!(error, DynamicFamilyError::Reserved { name, .. } if name == "tcl")
        ));
        assert!(
            outcome
                .rejected
                .iter()
                .any(|error| matches!(error, DynamicFamilyError::UnknownFamily { .. }))
        );
        let _ = register_dynamic_families(Vec::new(), Vec::new());
    }
}
