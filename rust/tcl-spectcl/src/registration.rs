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
//! - [`crate::loader::PackDialect`] blocks register as **families**, not
//!   identities: [`crate::dialect_conversion`] converts a validated block
//!   to runtime [`tcl_dialect::model::DynamicFamily`] data and binds it
//!   as the core of the pack's own environments. They are counted here so
//!   a caller can report the two halves separately.
//!
//! ## The production wiring
//!
//! Two entry points, for two shapes of caller:
//!
//! - [`register_pack_environments`] registers **one** pack's blocks,
//!   transactionally, at a stated tier. It is the seam a test or a tool
//!   holding a single [`Pack`] uses; nothing it registers ever retires.
//! - [`publish_pack_set`] registers **the** loaded set — every discovered
//!   workspace, user, and bundled pack at once — and is what
//!   [`crate::bundled::set_active`] calls on every reload. Because the
//!   caller hands over the whole set, a pack that has gone from the
//!   workspace has its environments retired by the same call that
//!   re-registers the packs that are still there, and the extension
//!   routing detection resolves through is republished with them.
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

use tcl_registry::model::{
    EnvironmentExtension, EnvironmentRegistrationError, EnvironmentSource, SyncOutcome,
};

use crate::discovery::Tier;
use crate::loader::{Pack, PackEnvironment, PackEnvironmentTier};
use crate::pack::{MergedPack, PackSet};

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
    /// `dialect` blocks the pack carries that this single-pack seam does
    /// not convert.
    ///
    /// Converting a block to live family data needs the *pack's* name
    /// (the namespace of the id it converts under), so it happens at the
    /// set seam — [`publish_pack_set`] — not here. A caller holding one
    /// pack reports the count and calls
    /// [`crate::dialect_conversion::to_dynamic_family`] itself if it
    /// wants the families.
    pub dialects_deferred: usize,
}

/// The one loaded pack set's environments, as the live registry sees
/// them — the production wiring of the seam above.
///
/// [`register_pack_set`] hands the *whole* set over, so the answer covers
/// retirement as well as registration: what the set no longer carries
/// stops resolving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSetRegistration {
    /// The live registry generation after the call.
    pub generation: u64,
    /// Whether the call actually moved the registry. `false` is the
    /// common reload answer — the same packs, so the same environments,
    /// and deliberately the same generation.
    pub changed: bool,
    /// Environment declarations live from this set.
    pub declared: usize,
    /// `-extend` contributions live from this set.
    pub extended: usize,
    /// Environments a previous set declared that this one does not — a
    /// deleted or renamed pack's environments retiring.
    pub retired: usize,
    /// `dialect` blocks across the set that converted to live family
    /// data (they are families, not identities, so they register through
    /// [`tcl_dialect::model::register_dynamic_families`], not the
    /// environment registry).
    pub dialects: usize,
    /// Environments bound to a pack-declared core.
    pub dynamic_cores: usize,
    /// `dialect` blocks that did not convert, each with the reason.
    pub dialects_refused: Vec<DialectRejection>,
    /// Packs whose environments were refused, each with the rule broken.
    /// Every other pack in the set still registered.
    pub rejected: Vec<PackRejection>,
}

/// One `dialect` block that did not become live family data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialectRejection {
    /// The declaring pack's `speclib` name.
    pub pack: String,
    /// What the block said, and why that was not enough.
    pub reason: String,
}

/// One pack whose environments did not register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRejection {
    /// The pack's `speclib` name.
    pub pack: String,
    /// Why the registry refused it.
    pub error: EnvironmentRegistrationError,
}

/// The source id a pack registers under: stable across reloads (so a
/// re-registration replaces rather than stacks) and unique per pack per
/// tier (so the same pack name at two tiers cannot silently merge).
fn source_id(pack: &MergedPack) -> String {
    format!("{}:{}", pack.tier.label(), pack.name)
}

/// Register **the** loaded pack set's environments into the live
/// registry, retiring whatever a previous set registered and this one
/// does not.
///
/// This is the production wiring: the one publish point every consumer in
/// the process resolves against ([`crate::bundled::set_active`]) calls it
/// on each reload, so a workspace or user pack's `environment` blocks are
/// live in the running server, under the §6.4 trust lattice, with the
/// generation bump the per-context caches already key on.
///
/// One pack's violation is that pack's problem: it is reported in
/// [`PackSetRegistration::rejected`] and its blocks are dropped, while
/// every other pack in the set registers. Failing the whole set would
/// mean one malformed workspace pack silently un-registered every other
/// pack's environments.
#[must_use]
pub fn register_pack_set(packs: &PackSet) -> PackSetRegistration {
    let mut sources = Vec::with_capacity(packs.packs.len());
    let mut rejected = Vec::new();
    for pack in &packs.packs {
        let tier = PackEnvironmentTier::of(pack.tier);
        if let Some(error) = untrusted_compiled_extension(&pack.environments, pack.tier) {
            rejected.push(PackRejection {
                pack: pack.name.clone(),
                error,
            });
            continue;
        }
        let (definitions, extensions) = split(&pack.environments, tier);
        if definitions.is_empty() && extensions.is_empty() {
            continue;
        }
        sources.push(EnvironmentSource {
            id: source_id(pack),
            definitions,
            extensions,
        });
    }
    let by_source: std::collections::HashMap<&str, &str> = packs
        .packs
        .iter()
        .map(|pack| (source_id_key(pack), pack.name.as_str()))
        .collect();
    let SyncOutcome {
        generation,
        changed,
        declared,
        extended,
        retired,
        rejected: refused,
    } = tcl_registry::model::sync_environment_sources(sources);
    for refusal in refused {
        rejected.push(PackRejection {
            pack: by_source
                .get(refusal.source.as_str())
                .map_or_else(|| refusal.source.clone(), |name| (*name).to_owned()),
            error: refusal.error,
        });
    }
    let (dialects, dynamic_cores, dialects_refused) = register_pack_dialects(packs);
    PackSetRegistration {
        generation,
        changed,
        declared,
        extended,
        retired,
        dialects,
        dynamic_cores,
        dialects_refused,
        rejected,
    }
}

/// Convert every loaded `dialect` block to runtime family data and sync
/// it, together with the core bindings of the environments that ride one.
///
/// A sync, like the environment channel: a dialect whose pack has left the
/// workspace retires. The environments' bindings are registered in the
/// same call because a binding without its family is not a fact — the
/// model refuses it, and reporting that once here is clearer than
/// discovering it at resolve time.
fn register_pack_dialects(packs: &PackSet) -> (usize, usize, Vec<DialectRejection>) {
    let mut families = Vec::new();
    let mut cores = Vec::new();
    let mut refused = Vec::new();
    for pack in &packs.packs {
        let tier = PackEnvironmentTier::of(pack.tier);
        for dialect in &pack.dialects {
            match crate::dialect_conversion::to_dynamic_family(dialect, &pack.name, tier) {
                Ok(family) => families.push(family),
                Err(error) => refused.push(DialectRejection {
                    pack: pack.name.clone(),
                    reason: error.to_string(),
                }),
            }
        }
        for environment in &pack.environments {
            if let Some(core) = crate::dialect_conversion::to_dynamic_core(environment, &pack.name)
            {
                cores.push(core);
            }
        }
    }
    // Q6: the surface rosters ride the same set-wide sync, and the
    // compiled-in core surfaces are folded in on every publication — the
    // model-side call replaces the whole store, so a set that declared no
    // roster of its own must still hand Jim's back or it would retire it.
    let mut rosters = crate::core_surfaces::builtin_rosters();
    for pack in &packs.packs {
        let provenance = PackEnvironmentTier::of(pack.tier).provenance();
        rosters.extend(crate::surface_roster_conversion::to_inherited_surfaces(
            &pack.surface_rosters,
            provenance,
        ));
    }
    let roster_outcome = tcl_dialect::model::register_inherited_surfaces(rosters);
    refused.extend(
        roster_outcome
            .rejected
            .iter()
            .map(|error| DialectRejection {
                pack: String::new(),
                reason: error.to_string(),
            }),
    );
    let outcome = tcl_dialect::model::register_dynamic_families(families, cores);
    refused.extend(outcome.rejected.iter().map(|error| DialectRejection {
        pack: String::new(),
        reason: error.to_string(),
    }));
    (outcome.families, outcome.cores, refused)
}

/// Register the set's environments and republish the extension routing
/// that follows from them — the whole publication one loaded pack set
/// makes to the running process.
///
/// The routing half is what turns a registered environment into a
/// *reachable* one: detection resolves a document by extension through
/// [`tcl_registry::dialects::dialect_from_extension`], so a pack that
/// declares `environment vivaldi-shell-tcl { … file_extension vsh … }`
/// only routes `.vsh` documents to it once that pair is published. An
/// explicit `file_extension … -dialect D` row still wins over an
/// environment's own claim on the same extension: it is the more specific
/// statement, and it is the one 1.x packs already use.
#[must_use]
pub fn publish_pack_set(packs: &PackSet) -> PackSetRegistration {
    let registration = register_pack_set(packs);
    tcl_registry::dialects::register_pack_extension_dialects(extension_routes(packs));
    registration
}

/// Retire every environment the pack channel registered — the answer to
/// "no pack set is published any more".
#[must_use]
pub fn retire_pack_environments() -> PackSetRegistration {
    let outcome = tcl_registry::model::sync_environment_sources(Vec::new());
    let _ = tcl_dialect::model::register_dynamic_families(Vec::new(), Vec::new());
    // The compiled-in core surfaces are not the pack channel's to retire.
    let _ =
        tcl_dialect::model::register_inherited_surfaces(crate::core_surfaces::builtin_rosters());
    PackSetRegistration {
        generation: outcome.generation,
        changed: outcome.changed,
        declared: 0,
        extended: 0,
        retired: outcome.retired,
        dialects: 0,
        dynamic_cores: 0,
        dialects_refused: Vec::new(),
        rejected: Vec::new(),
    }
}

/// Every `(extension, environment)` routing pair the loaded set claims:
/// the explicit `-dialect` rows first, then each `environment` block's
/// own `file_extension` rows, first claim winning.
///
/// A **pure function of the pack set** — the environment name is the one
/// the block wrote, not one resolved through the live registry. That is
/// deliberate: the pack merge (`crate::pack`) publishes this on *every*
/// merge, including a reload whose content turned out identical and therefore
/// never reaches [`publish_pack_set`], and a routing table that depended
/// on registration order would lose its environment rows on exactly those
/// reloads. The name a row routes to goes through the ingress anyway, so
/// an alias resolves and an environment that failed to register lands on
/// the lenient fallback rather than misrouting.
///
/// The names are interned because the detection table is keyed by
/// `&'static str` — bounded by the number of distinct pack-declared
/// environments a process ever loads, and shared with the loader's own
/// interner, so reloading the same pack leaks nothing new.
#[must_use]
pub fn extension_routes(packs: &PackSet) -> Vec<(String, &'static str)> {
    let mut routes = packs.extension_dialects();
    for pack in &packs.packs {
        for environment in &pack.environments {
            let environment_id = crate::loader::leak_str(&environment.id);
            for claim in &environment.file_extensions {
                let extension = claim.extension.as_ref();
                if routes.iter().any(|(prior, _)| prior == extension) {
                    continue;
                }
                routes.push((extension.to_owned(), environment_id));
            }
        }
    }
    routes
}

/// [`source_id`] as a borrowed key, for the id → pack-name map above.
/// Leaked once per `(tier, name)` pair, which is bounded by the number of
/// distinct packs a process ever loads.
fn source_id_key(pack: &MergedPack) -> &'static str {
    crate::loader::leak_str(&source_id(pack))
}

/// The E-R2 tier pre-check, per pack: a workspace or studio-override pack
/// may not extend a compiled environment.
fn untrusted_compiled_extension(
    environments: &[PackEnvironment],
    tier: Tier,
) -> Option<EnvironmentRegistrationError> {
    if !matches!(tier, Tier::Workspace | Tier::StudioOverride) {
        return None;
    }
    environments
        .iter()
        .filter(|environment| environment.extends)
        .find_map(|environment| {
            crate::loader::reserved_environment_name(&environment.id).map(|reserved| {
                EnvironmentRegistrationError::UntrustedExtension {
                    base: reserved,
                    provenance: PackEnvironmentTier::of(tier).provenance(),
                }
            })
        })
}

/// The declarations and `-extend` contributions of one pack's blocks.
fn split(
    environments: &[PackEnvironment],
    tier: PackEnvironmentTier,
) -> (
    Vec<tcl_dialect::model::EnvironmentDefinition>,
    Vec<EnvironmentExtension>,
) {
    let definitions = environments
        .iter()
        .filter(|environment| !environment.extends)
        .map(|environment| environment.to_definition(tier))
        .collect();
    let extensions = environments
        .iter()
        .filter(|environment| environment.extends)
        .map(|environment| environment.to_extension(tier))
        .collect();
    (definitions, extensions)
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
    if let Some(error) = untrusted_compiled_extension(environments, tier) {
        return Err(error);
    }
    let (definitions, extensions) = split(environments, pack_tier);
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
