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

//! **`PackDialect` → runtime family data** — the P3 rider on §6.2's
//! `dialect` block (redesign Q1's endgame direction).
//!
//! The loader already parses a `dialect NAME { … }` block against the
//! closed axis vocabulary, projects its axes onto a
//! [`LexerGrammar`](tcl_dialect::LexerGrammar)
//! ([`PackDialect::to_grammar`]), and applies §2's classification gate —
//! a block whose grammar equals a compiled family release is rejected at
//! *load*, pointing at `environment`. What was left was the conversion:
//! turning what survives into live [`DynamicFamily`] data a pack-declared
//! environment can name as its core.
//!
//! ## The shape
//!
//! - **Namespaced ids.** A converted family is `PACK/DIALECT`
//!   (`spicegentcl/ngspice`) — §3.3's pack-name-prefixed scheme for
//!   third-party ids, applied to grammars because §6.2 says grammar
//!   declarations sit at the top of the trust lattice. Compiled family
//!   names are reserved; [`tcl_dialect::model::register_dynamic_families`]
//!   refuses a claim on one whatever tier it came from.
//! - **One grammar per ladder.** The block's `axis` rows are declared for
//!   the block, not per release, so every release on the declared ladder
//!   carries the block's grammar. Per-release axis values are a parser
//!   change (an `axis` row inside a `release` row), not a conversion one.
//! - **A ladder is required.** A `dialect` with no `release` row has
//!   nowhere to place a core, so it does not convert. The notice says so.
//! - **Trust rides the tier**, exactly as an environment's does.
//!
//! ## The boundary
//!
//! A converted family is *not* a [`tcl_dialect::model::Family`] variant,
//! and cannot be: that enum is closed and its `grammar()` is a `const fn`
//! over ladder ordinals. So an environment riding a pack-declared core
//! carries `core: None` plus a [`DynamicCore`] binding, and the grammar
//! is reachable through
//! [`tcl_dialect::model::dynamic_core_grammar`]. What consumes a
//! `LexerGrammar` today is `tcl_lexer::LexerConfig`, built from a
//! `&'static DialectProfile` the ingress hands out of a compiled table —
//! so the last step, handing the analysis path a grammar that is not a
//! compiled profile's, waits on the `DialectProfile` re-type (ledger C1).
//! See [`tcl_dialect::model::dynamic`] for the same boundary stated from
//! the model side.

use tcl_dialect::model::{Family};
use std::sync::Arc;

use tcl_dialect::model::{DynamicCore, DynamicFamily, DynamicFamilyId, DynamicRelease};

use crate::loader::{PackDialect, PackEnvironment, PackEnvironmentTier};

/// The namespaced id a pack's dialect converts under: `PACK/DIALECT`.
#[must_use]
pub fn family_id(pack: &str, dialect: &str) -> DynamicFamilyId {
    DynamicFamilyId::new(&format!("{pack}/{dialect}"))
}

/// Why one `dialect` block did not convert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// The block declares no `release` row, so it has no ladder.
    NoLadder(String),
    /// The block's axes name a value with no Rust backing (a reserved
    /// `jim*` spelling), so no grammar can be built from it yet.
    NoGrammar(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLadder(name) => write!(
                f,
                "`dialect {name}` declares no `release` row, so an environment has no \
                 release to place a core on; the dialect is carried but not registered"
            ),
            Self::NoGrammar(name) => write!(
                f,
                "`dialect {name}` sets an axis value this build reserves but cannot yet \
                 build a lexer with; the dialect is carried but not registered"
            ),
        }
    }
}

impl std::error::Error for ConversionError {}

/// Convert one loaded `dialect` block to runtime family data.
///
/// # Errors
///
/// [`ConversionError`] when the block has no ladder to place a core on,
/// or its axes project onto no buildable grammar.
pub fn to_dynamic_family(
    dialect: &PackDialect,
    pack: &str,
    tier: PackEnvironmentTier,
) -> Result<DynamicFamily, ConversionError> {
    let Some(grammar) = dialect.to_grammar() else {
        return Err(ConversionError::NoGrammar(dialect.name.clone()));
    };
    if dialect.releases.is_empty() {
        return Err(ConversionError::NoLadder(dialect.name.clone()));
    }
    Ok(DynamicFamily {
        id: family_id(pack, &dialect.name),
        display_name: Arc::from(dialect.name.as_str()),
        releases: dialect
            .releases
            .iter()
            .map(|release| DynamicRelease {
                name: Arc::from(release.release.as_str()),
                build: release.build,
                grammar,
            })
            .collect(),
        provenance: tier.provenance(),
    })
}

/// The [`DynamicCore`] binding one environment's pack-declared `core` row
/// describes, when it has one.
///
/// The loader has already proved the row names a `dialect` block of the
/// same pack (`finish_pack_cores` rejects the environment otherwise), so
/// this is a total projection onto the namespaced id.
#[must_use]
pub fn to_dynamic_core(environment: &PackEnvironment, pack: &str) -> Option<DynamicCore> {
    let core = environment.pack_core.as_ref()?;
    Some(DynamicCore {
        environment: environment.id.clone(),
        family: family_id(pack, &core.dialect),
        release: Arc::from(core.release.as_str()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate_pack;

    /// A `dialect` block converts to a namespaced family whose releases
    /// carry the block's grammar, and the environment riding it converts
    /// to the matching core binding.
    #[test]
    fn a_dialect_block_converts_to_a_namespaced_family_and_its_core() {
        let pack = evaluate_pack(
            "speclib picolpack 2.0 {\n\
             dialect picol2 {\n\
             \x20   release 2.0\n\
             \x20   release 2.1 -build Unknown\n\
             \x20   axis expand_syntax off\n\
             \x20   axis braced_var first-close\n\
             \x20   axis numbers tcl84\n\
             }\n\
             environment picol-shell {\n\
             \x20   core picol2 2.1\n\
             \x20   file_extension pcl\n\
             }\n\
             }\n",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        let family = to_dynamic_family(&pack.surface[0], &pack.name, PackEnvironmentTier::User)
            .expect("the block converts");
        assert_eq!(family.id.as_str(), "picolpack/picol2");
        assert_eq!(family.releases.len(), 2);
        assert!(!family.newest().grammar.expand_syntax);
        assert_eq!(family.newest().name.as_ref(), "2.1");
        assert_eq!(
            family.newest().build,
            tcl_dialect::model::BuildProfileId::Unknown
        );

        let core = to_dynamic_core(&pack.environments[0], &pack.name).expect("the core binding");
        assert_eq!(core.environment, "picol-shell");
        assert_eq!(core.family.as_str(), "picolpack/picol2");
        assert_eq!(core.release.as_ref(), "2.1");
        // The compiled half stays empty: a pack-declared family is not a
        // `Family` variant, so there is no `CoreProfileSelector` to hold.
        assert!(pack.environments[0].core.is_none());
    }

    /// A ladderless `dialect` carries but does not convert — an
    /// environment has no release to place a core on.
    #[test]
    fn a_ladderless_dialect_does_not_convert() {
        let pack = evaluate_pack(
            "speclib picolpack 2.0 {\n\
             dialect axesonly {\n\
             \x20   axis expand_syntax off\n\
             \x20   axis numbers tcl84\n\
             }\n\
             }\n",
        );
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert_eq!(
            to_dynamic_family(&pack.surface[0], &pack.name, PackEnvironmentTier::User),
            Err(ConversionError::NoLadder("axesonly".to_owned()))
        );
    }

    /// A `core` row naming neither a compiled family nor a `dialect` this
    /// pack declares rejects the environment — the §6.1 semantic class,
    /// resolved after the whole file is read so a forward reference works.
    #[test]
    fn a_core_row_naming_no_declared_dialect_rejects_the_environment() {
        let pack = evaluate_pack(
            "speclib picolpack 2.0 {\n\
             environment picol-shell {\n\
             \x20   core nosuchdialect 2.0\n\
             }\n\
             }\n",
        );
        assert!(pack.environments.is_empty(), "{:?}", pack.environments);
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains("neither a compiled family")),
            "{:?}",
            pack.notices
        );

        // …and a release the ladder does not carry is refused the same way.
        let pack = evaluate_pack(
            "speclib picolpack 2.0 {\n\
             environment picol-shell {\n\
             \x20   core picol2 9.9\n\
             }\n\
             dialect picol2 {\n\
             \x20   release 2.0\n\
             \x20   axis expand_syntax off\n\
             \x20   axis numbers tcl84\n\
             }\n\
             }\n",
        );
        assert!(pack.environments.is_empty(), "{:?}", pack.environments);
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.message.contains("is not a `release` row")),
            "{:?}",
            pack.notices
        );
    }
}
