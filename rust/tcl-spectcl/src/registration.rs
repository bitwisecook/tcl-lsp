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

//! **The pack-side environment registration seam** (P2-H, deliverable E):
//! a loaded pack's `environment` blocks — declarations and `-extend`
//! contributions — enter the live [`EnvironmentRegistry`] through
//! [`tcl_registry::model::register_environments`], under the §6.4 trust
//! lattice, with invalidation riding the registry-generation machinery.
//!
//! ## What registers, and what does not (yet)
//!
//! - [`PackEnvironment`] declarations register as
//!   [`EnvironmentDefinition`]s at the pack tier's provenance;
//!   `-extend` blocks register as [`EnvironmentExtension`]s.
//! - [`crate::loader::PackDialect`] blocks do **not** register:
//!   conversion of pack-declared axes to live `Family` data is P3+
//!   (redesign §6.2), and the E-R2 evaluation gate already refuses
//!   untrusted `dialect` blocks outright. They are counted in the
//!   outcome so a caller can report them honestly.
//!
//! ## Trust (E-R2, reused)
//!
//! The tier maps to provenance through
//! [`PackEnvironmentTier::provenance`], and the registry-side seam
//! enforces the lattice: a reserved-name claim fails the whole
//! registration with the provenance-naming error, and an untrusted tier
//! (workspace, studio override) cannot extend a compiled environment.
//! The loader has already rejected declarations *claiming* compiled
//! names at parse; the registry-side check is the second lock on the
//! same door, for callers that construct definitions without the loader.
//!
//! [`EnvironmentRegistry`]: tcl_dialect::model::EnvironmentRegistry
//! [`EnvironmentDefinition`]: tcl_dialect::model::EnvironmentDefinition
//! [`EnvironmentExtension`]: tcl_registry::model::EnvironmentExtension

use tcl_registry::model::{EnvironmentExtension, EnvironmentRegistrationError};

use crate::discovery::Tier;
use crate::loader::{Pack, PackEnvironment, PackEnvironmentTier};

/// What one registration call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationOutcome {
    /// The live registry generation the call moved to, when anything
    /// was registered.
    pub generation: Option<u64>,
    /// Environment declarations registered.
    pub declared: usize,
    /// `-extend` contributions registered.
    pub extended: usize,
    /// `dialect` blocks the pack carries that this seam deliberately
    /// does not register (P3+).
    pub dialects_deferred: usize,
}

/// Register one loaded pack's environments into the live registry at
/// `tier`, transactionally: either every block lands or the error names
/// the violation and the registry is untouched.
///
/// # Errors
///
/// [`EnvironmentRegistrationError`] from the registry-side seam — a
/// reserved-name claim (with the claiming provenance), an untrusted
/// extension of a compiled base, an unknown extension base, or a §3.3
/// collision.
pub fn register_pack_environments(
    pack: &Pack,
    tier: Tier,
) -> Result<RegistrationOutcome, EnvironmentRegistrationError> {
    register_environments(&pack.environments, pack.dialects.len(), tier)
}

/// [`register_pack_environments`] over bare blocks, for callers holding
/// the blocks rather than a whole [`Pack`].
pub fn register_environments(
    environments: &[PackEnvironment],
    dialect_blocks: usize,
    tier: Tier,
) -> Result<RegistrationOutcome, EnvironmentRegistrationError> {
    let pack_tier = PackEnvironmentTier::of(tier);
    // The E-R2 pre-check, reusing the loader's own reserved-name
    // question: a workspace or studio-override pack may not extend a
    // compiled environment, whatever provenance its tier maps to — the
    // loader tier is the trust boundary the evaluation gate enforces, and
    // this seam enforces the same one for CST-loaded packs.
    if matches!(tier, Tier::Workspace | Tier::StudioOverride) {
        for environment in environments.iter().filter(|e| e.extends) {
            if let Some(reserved) = crate::loader::reserved_environment_name(&environment.id) {
                return Err(EnvironmentRegistrationError::UntrustedExtension {
                    base: reserved,
                    provenance: pack_tier.provenance(),
                });
            }
        }
    }
    let definitions: Vec<_> = environments
        .iter()
        .filter(|environment| !environment.extends)
        .map(|environment| environment.to_definition(pack_tier))
        .collect();
    let extensions: Vec<EnvironmentExtension> = environments
        .iter()
        .filter(|environment| environment.extends)
        .map(|environment| environment.to_extension(pack_tier))
        .collect();
    let declared = definitions.len();
    let extended = extensions.len();
    if declared == 0 && extended == 0 {
        return Ok(RegistrationOutcome {
            generation: None,
            declared,
            extended,
            dialects_deferred: dialect_blocks,
        });
    }
    let generation = tcl_registry::model::register_environments(definitions, extensions)?;
    Ok(RegistrationOutcome {
        generation: Some(generation),
        declared,
        extended,
        dialects_deferred: dialect_blocks,
    })
}
