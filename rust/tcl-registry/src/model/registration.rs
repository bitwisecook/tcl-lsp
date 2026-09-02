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

//! **Live environment registration** — the P2 seam the centralisation
//! contract's §1.1 documents: pack- or configuration-declared
//! environments join the one [`EnvironmentRegistry`] the ingress
//! ([`crate::model::ingress`]) resolves through, under the redesign's
//! §6.4 trust lattice, with invalidation riding the existing
//! generation machinery.
//!
//! ## Shape
//!
//! The live registry is a swapped-in [`Arc<EnvironmentRegistry>`]:
//! generation 0 is the compiled seed set, and every successful
//! [`register_environments`] call rebuilds the whole registry —
//! compiled definitions, plus every extension applied, plus every
//! dynamic definition — at the **next** generation. Rebuilding from
//! the seed on every registration is what makes registration
//! transactional (a collision rolls the whole call back, the previous
//! registry keeps serving) and idempotent (re-registering a reloaded
//! pack replaces its previous definitions instead of stacking them).
//!
//! ## Invalidation
//!
//! Nothing here touches the per-context generation cache directly.
//! [`tcl_dialect::model::EnvironmentIdentity`] carries the registry
//! generation, the
//! generation cache ([`crate::model::assembly`]) keys on the identity,
//! and [`crate::model::ingress::resolve_environment`] resolves against
//! the live registry — so a registration bumps the generation, every
//! subsequent resolve carries the new generation, and stale
//! [`crate::model::ContextRegistry`] values simply stop being reachable
//! through the ingress and drop with their last holder. That is the
//! "overlay/generation machinery" doing the invalidation, not a second
//! cache-clearing protocol.
//!
//! ## Trust (§6.4, reusing the E-R2 vocabulary)
//!
//! - **Compiled names are reserved.** A registered definition claiming a
//!   compiled canonical id or alias fails the whole call with
//!   [`EnvironmentRegistrationError::Reserved`], naming the claiming
//!   definition's provenance — the registration-time twin of the
//!   loader-side E-R2 gate.
//! - **Untrusted tiers cannot alter compiled environments.** An
//!   [`EnvironmentExtension`] whose base is a compiled definition is
//!   refused when its provenance is `WorkspaceUntrusted` or
//!   `StudioOverride` — §6.4's "overriding a canonical environment
//!   requires explicit trusted opt-in". Extending a *pack-declared*
//!   environment is an ordinary addition at any tier.
//! - **Additions are open.** A new, non-reserved environment id may be
//!   declared from any tier, untrusted included — a new identity is an
//!   addition, not an override.
//!
//! ## Sources, and how a removed pack's environments retire
//!
//! [`register_environments`] is the *anonymous* channel: what it
//! registers stays registered until something re-registers the same id.
//! That is the right shape for a one-off declaration, and the wrong one
//! for a **set** that is republished whenever the workspace changes — a
//! pack deleted from `.tcl-lsp/` would keep its environments alive for
//! the life of the process.
//!
//! [`sync_environment_sources`] is that second channel: the caller hands
//! over the whole set, keyed by [`EnvironmentSource::id`], and the
//! source-keyed half of the dynamic state is **replaced**. A source that
//! is no longer in the set retires with the rebuild; a source that is
//! still there re-registers idempotently. One malformed source does not
//! disable the rest: the set is validated whole, each source that breaks
//! it is reported by id and dropped, and the rest register — which is
//! what keeps one broken workspace pack from taking every other pack's
//! environments with it.
//! The anonymous channel is untouched by a sync, and vice versa.

use std::sync::{Arc, Mutex, OnceLock};

use tcl_dialect::model::{
    EnvironmentDefinition, EnvironmentRegistry, EnvironmentRegistryError, FileExtensionClaim,
    PackagePlacement, Provenance,
};

/// An additive extension of an existing environment: detection facts and
/// package placements contributed to a base definition another source
/// declared (compiled or registered). Nothing here can *remove* or
/// *weaken* a base fact — the fields are append-only by construction,
/// which is what makes an extension a different operation from a
/// definition and lets the trust rule treat the two differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentExtension {
    /// The canonical id (or alias) of the environment being extended.
    pub base: String,
    /// `file_extension` detection claims to add.
    pub file_extensions: Vec<FileExtensionClaim>,
    /// `filename` detection claims to add.
    pub filenames: Vec<Arc<str>>,
    /// `signature` detection claims to add.
    pub content_signatures: Vec<Arc<str>>,
    /// Package placements (ambient or hosted) to add. A placement naming
    /// a package the base already places is dropped — the base's claim
    /// wins, so an extension can never re-version a compiled placement.
    pub placements: Vec<PackagePlacement>,
    /// The §6.4 trust class of the declaring source.
    pub provenance: Provenance,
}

/// Why a [`register_environments`] call registered nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentRegistrationError {
    /// A registered definition claims a compiled canonical id or alias.
    Reserved {
        /// The reserved spelling that was claimed.
        name: String,
        /// The canonical id of the claiming definition.
        claimed_by: String,
        /// The claiming definition's trust class.
        provenance: Provenance,
    },
    /// An extension of a compiled environment came from an untrusted
    /// tier.
    UntrustedExtension {
        /// The compiled environment the extension targets.
        base: String,
        /// The extension's trust class.
        provenance: Provenance,
    },
    /// An extension names a base no registry generation holds.
    UnknownExtensionBase {
        /// The unresolvable base spelling.
        base: String,
    },
    /// The rebuilt registry violated the §3.3 collision contract in a
    /// way the rules above did not already name.
    Collision(EnvironmentRegistryError),
}

/// The §6.4 label a trust class prints under.
#[must_use]
pub fn provenance_label(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::BuiltIn => "built-in",
        Provenance::BundledPack => "bundled",
        Provenance::User => "user",
        Provenance::WorkspaceTrusted => "trusted workspace",
        Provenance::WorkspaceUntrusted => "untrusted workspace",
        Provenance::StudioOverride => "studio override",
        Provenance::Document => "document",
    }
}

impl std::fmt::Display for EnvironmentRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reserved {
                name,
                claimed_by,
                provenance,
            } => write!(
                f,
                "environment `{claimed_by}` claims `{name}`, a compiled or bundled \
                 environment name, and it registers from the {} tier; a pack-declared \
                 environment may not claim a reserved id or alias, so nothing from this \
                 registration is loaded (design §3.3/§6.4, E-R2)",
                provenance_label(*provenance)
            ),
            Self::UntrustedExtension { base, provenance } => write!(
                f,
                "the extension of compiled environment `{base}` registers from the {} \
                 tier; altering a canonical environment requires a trusted tier, so \
                 nothing from this registration is loaded (design §6.4, E-R2)",
                provenance_label(*provenance)
            ),
            Self::UnknownExtensionBase { base } => write!(
                f,
                "the extension names `{base}`, which is not a known environment in \
                 any registry generation; nothing from this registration is loaded"
            ),
            Self::Collision(error) => write!(
                f,
                "the registered environments collide with the registry: {error}; \
                 nothing from this registration is loaded"
            ),
        }
    }
}

impl std::error::Error for EnvironmentRegistrationError {}

/// Everything one publisher — one pack, one configuration file —
/// declares, under an id stable across reloads.
///
/// The unit [`sync_environment_sources`] replaces: the same id
/// re-published replaces its previous contribution, and an id absent
/// from a sync retires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentSource {
    /// The publisher's stable id — a pack's tier-qualified name, a
    /// configuration file's path. Reported verbatim when the source is
    /// rejected, so the caller can attach the notice to the right file.
    pub id: String,
    /// The environment declarations this source contributes.
    pub definitions: Vec<EnvironmentDefinition>,
    /// The `-extend` contributions this source makes.
    pub extensions: Vec<EnvironmentExtension>,
}

/// One source a sync refused, with the rule it broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedSource {
    /// The refused source's [`EnvironmentSource::id`].
    pub source: String,
    /// Why it was refused.
    pub error: EnvironmentRegistrationError,
}

/// What one [`sync_environment_sources`] call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    /// The live registry generation after the call — the previous one
    /// when nothing changed.
    pub generation: u64,
    /// Whether the sync actually rebuilt the registry. `false` means the
    /// set was byte-identical to the registered one, so the generation
    /// deliberately did **not** move: a reload that found the same packs
    /// must not invalidate every downstream generation cache.
    pub changed: bool,
    /// Definitions registered across every accepted source.
    pub declared: usize,
    /// `-extend` contributions registered across every accepted source.
    pub extended: usize,
    /// Environment ids that were registered before this call and are not
    /// any more — a deleted pack's environments retiring.
    pub retired: usize,
    /// Sources refused, each with the rule it broke. The rest still
    /// registered.
    pub rejected: Vec<RejectedSource>,
}

/// The dynamic half of the live registry: what registrations have added
/// beyond the compiled seed. Rebuilt into a fresh [`EnvironmentRegistry`]
/// on every successful call.
#[derive(Debug, Clone, Default)]
struct DynamicState {
    /// Registered definitions, keyed by canonical id — a re-registration
    /// of the same id replaces the previous value (a pack reload).
    definitions: Vec<EnvironmentDefinition>,
    /// Registered extensions. Exact duplicates are dropped, so reloading
    /// a pack does not stack its detection rows.
    extensions: Vec<EnvironmentExtension>,
    /// The source-keyed half: replaced wholesale by
    /// [`sync_environment_sources`], so an absent source retires.
    sources: Vec<EnvironmentSource>,
}

/// The registration lock and the dynamic state behind it.
static STATE: OnceLock<Mutex<DynamicState>> = OnceLock::new();

/// The live registry the ingress resolves against.
static LIVE: OnceLock<Mutex<Arc<EnvironmentRegistry>>> = OnceLock::new();

fn live_cell() -> &'static Mutex<Arc<EnvironmentRegistry>> {
    LIVE.get_or_init(|| Mutex::new(Arc::new(EnvironmentRegistry::compiled())))
}

/// The current live registry — the compiled seed at generation 0 until a
/// registration lands, then every registration's rebuilt registry at the
/// next generation.
#[must_use]
pub fn live_environments() -> Arc<EnvironmentRegistry> {
    Arc::clone(&live_cell().lock().expect("live environment registry lock"))
}

/// Whether `provenance` is one of the untrusted tiers E-R2 gates.
fn untrusted(provenance: Provenance) -> bool {
    matches!(
        provenance,
        Provenance::WorkspaceUntrusted | Provenance::StudioOverride | Provenance::Document
    )
}

/// Apply one extension to `definition`, additively and idempotently:
/// detection claims already present (and placements for packages the base
/// already places) are dropped rather than duplicated.
fn extend(definition: &mut EnvironmentDefinition, extension: &EnvironmentExtension) {
    for claim in &extension.file_extensions {
        if !definition
            .server_detection
            .file_extensions
            .iter()
            .any(|prior| prior.extension == claim.extension)
        {
            definition
                .server_detection
                .file_extensions
                .push(claim.clone());
        }
    }
    for name in &extension.filenames {
        if !definition
            .server_detection
            .filenames
            .iter()
            .any(|prior| prior == name)
        {
            definition.server_detection.filenames.push(Arc::clone(name));
        }
    }
    for signature in &extension.content_signatures {
        if !definition
            .server_detection
            .content_signatures
            .iter()
            .any(|prior| prior == signature)
        {
            definition
                .server_detection
                .content_signatures
                .push(Arc::clone(signature));
        }
    }
    for placement in &extension.placements {
        if !definition
            .expected_packages
            .iter()
            .any(|prior| prior.package == placement.package)
        {
            definition.expected_packages.push(placement.clone());
        }
    }
}

/// Assemble the registry one dynamic state describes, at `generation`.
///
/// The one place the trust gates and the rebuild order live, so the
/// anonymous channel ([`register_environments`]) and the source channel
/// ([`sync_environment_sources`]) cannot drift apart: compiled seed,
/// then the anonymous definitions, then each source's definitions in
/// source order, then every extension applied to whichever definition
/// owns its base spelling.
fn assemble(
    definitions: &[EnvironmentDefinition],
    extensions: &[EnvironmentExtension],
    sources: &[EnvironmentSource],
    generation: u64,
) -> Result<EnvironmentRegistry, EnvironmentRegistrationError> {
    let declared: Vec<&EnvironmentDefinition> = definitions
        .iter()
        .chain(sources.iter().flat_map(|source| source.definitions.iter()))
        .collect();
    let contributed: Vec<&EnvironmentExtension> = extensions
        .iter()
        .chain(sources.iter().flat_map(|source| source.extensions.iter()))
        .collect();

    // Trust gates before assembly, so the error names the rule rather
    // than a downstream collision.
    let compiled = EnvironmentRegistry::compiled();
    for extension in &contributed {
        let compiled_base = compiled.resolve(&extension.base);
        if let Some(base) = &compiled_base {
            if untrusted(extension.provenance) {
                return Err(EnvironmentRegistrationError::UntrustedExtension {
                    base: base.id.as_str().to_owned(),
                    provenance: extension.provenance,
                });
            }
        } else if !declared.iter().any(|definition| {
            definition.id.as_str() == extension.base
                || definition
                    .aliases
                    .iter()
                    .any(|alias| alias.as_ref() == extension.base)
        }) {
            return Err(EnvironmentRegistrationError::UnknownExtensionBase {
                base: extension.base.clone(),
            });
        }
    }

    let mut rebuilt = tcl_dialect::model::compiled_definitions();
    for definition in &declared {
        // A bundled pack restating an environment the compiled seed already
        // carries from that same pack (D17) replaces the seed row: the
        // on-disk pack is authoritative, and two rows would collide.
        let seeded = (definition.provenance == Provenance::BundledPack)
            .then(|| {
                rebuilt.iter().position(|seed| {
                    seed.provenance == Provenance::BundledPack && seed.id == definition.id
                })
            })
            .flatten();
        match seeded {
            Some(position) => rebuilt[position] = (*definition).clone(),
            None => rebuilt.push((*definition).clone()),
        }
    }
    for extension in &contributed {
        let base = rebuilt.iter_mut().find(|definition| {
            definition.id.as_str() == extension.base
                || definition
                    .aliases
                    .iter()
                    .any(|alias| alias.as_ref() == extension.base)
        });
        if let Some(definition) = base {
            extend(definition, extension);
        }
    }

    EnvironmentRegistry::new(rebuilt, generation).map_err(|error| match &error {
        EnvironmentRegistryError::ReservedName { name, claimed_by } => {
            // The violator, not merely a definition with that id: a bundled
            // pack restating its own seed row shares the id with the
            // intruder claiming it.
            let provenance = declared
                .iter()
                .filter(|definition| definition.id.as_str() == claimed_by)
                .find(|definition| {
                    tcl_dialect::model::reserved_against(name, definition.provenance).is_some()
                })
                .map_or(Provenance::WorkspaceUntrusted, |definition| {
                    definition.provenance
                });
            EnvironmentRegistrationError::Reserved {
                name: name.clone(),
                claimed_by: claimed_by.clone(),
                provenance,
            }
        }
        _ => EnvironmentRegistrationError::Collision(error),
    })
}

/// The live registry's current generation.
fn current_generation() -> u64 {
    live_cell()
        .lock()
        .expect("live environment registry lock")
        .generation()
}

/// Every canonical id a set of sources declares.
fn declared_ids(sources: &[EnvironmentSource]) -> Vec<String> {
    sources
        .iter()
        .flat_map(|source| source.definitions.iter())
        .map(|definition| definition.id.as_str().to_owned())
        .collect()
}

/// Assemble the live members of `sources` over the anonymous half.
fn assemble_live(
    state: &DynamicState,
    sources: &[EnvironmentSource],
    live: &[bool],
    generation: u64,
) -> Result<EnvironmentRegistry, EnvironmentRegistrationError> {
    let candidates: Vec<EnvironmentSource> = sources
        .iter()
        .zip(live)
        .filter(|&(_, &live)| live)
        .map(|(source, _)| source.clone())
        .collect();
    assemble(
        &state.definitions,
        &state.extensions,
        &candidates,
        generation,
    )
}

/// Which of `sources` a broken sync can keep, and why each of the rest
/// was refused.
///
/// The set is what failed — a rule can be broken several ways over, and
/// by pairs rather than by any one source — but [`SyncOutcome::rejected`]
/// must name files. So: peel latest-first until the survivors assemble,
/// which always terminates and lets the earlier of two sources claiming
/// one name keep its claim; then re-admit, earliest first, every peeled
/// source the survivors accept, repeating while any goes back, since an
/// `-extend` only becomes admissible once its declarer is among them.
/// What a source cannot rejoin is its own rule broken, and that is the
/// error reported against it.
fn triage(state: &DynamicState, sources: &[EnvironmentSource]) -> (Vec<bool>, Vec<RejectedSource>) {
    let mut live = vec![true; sources.len()];
    for index in (0..sources.len()).rev() {
        if assemble_live(state, sources, &live, 0).is_ok() {
            break;
        }
        live[index] = false;
    }

    let mut rejected = Vec::new();
    loop {
        let mut readmitted = false;
        rejected.clear();
        for index in 0..sources.len() {
            if live[index] {
                continue;
            }
            live[index] = true;
            match assemble_live(state, sources, &live, 0) {
                Ok(_) => readmitted = true,
                Err(error) => {
                    live[index] = false;
                    rejected.push(RejectedSource {
                        source: sources[index].id.clone(),
                        error,
                    });
                }
            }
        }
        if !readmitted {
            return (live, rejected);
        }
    }
}

/// Replace the source-keyed half of the live registry with `sources`.
///
/// The channel a **republished set** uses: workspace pack discovery hands
/// over every loaded pack on every reload, and this makes the live
/// registry say exactly that — new sources register, unchanged ones
/// re-register idempotently, and a source that has gone (a deleted pack)
/// retires with the rebuild.
///
/// Never fails as a whole. The candidate set is validated whole and each
/// source that breaks it is reported in [`SyncOutcome::rejected`] and
/// dropped while the rest register: a malformed workspace pack must not
/// take every other pack's environments down with it. A set
/// identical to the registered one is a no-op, generation included, so a
/// reload that found nothing new does not invalidate downstream caches.
#[must_use]
pub fn sync_environment_sources(sources: Vec<EnvironmentSource>) -> SyncOutcome {
    let state = STATE.get_or_init(|| Mutex::new(DynamicState::default()));
    let mut state = state.lock().expect("environment registration lock");

    // The candidate set is validated whole, never source by source: an
    // `-extend` may name a base a *later* source declares, and which of
    // the two pack discovery reached first must not decide whether the
    // pair loads.
    let (accepted, mut rejected) =
        if assemble(&state.definitions, &state.extensions, &sources, 0).is_ok() {
            (sources, Vec::new())
        } else {
            let (live, rejected) = triage(&state, &sources);
            let accepted = sources
                .into_iter()
                .zip(live)
                .filter_map(|(source, live)| live.then_some(source))
                .collect();
            (accepted, rejected)
        };

    let before = declared_ids(&state.sources);
    let after = declared_ids(&accepted);
    let retired = before.iter().filter(|id| !after.contains(id)).count();
    let declared = after.len();
    let extended: usize = accepted
        .iter()
        .map(|source| source.extensions.len())
        .sum::<usize>();

    if accepted == state.sources {
        return SyncOutcome {
            generation: current_generation(),
            changed: false,
            declared,
            extended,
            retired,
            rejected,
        };
    }

    let generation = current_generation() + 1;
    match assemble(&state.definitions, &state.extensions, &accepted, generation) {
        Ok(registry) => {
            *live_cell().lock().expect("live environment registry lock") = Arc::new(registry);
            state.sources = accepted;
            SyncOutcome {
                generation,
                changed: true,
                declared,
                extended,
                retired,
                rejected,
            }
        }
        Err(error) => {
            // Reachable only when the base registrations themselves break
            // a rule, which no candidate can repair. Keep the live registry
            // and report the rule against no source, rather than panicking.
            rejected.push(RejectedSource {
                source: String::new(),
                error,
            });
            SyncOutcome {
                generation: current_generation(),
                changed: false,
                declared: 0,
                extended: 0,
                retired: 0,
                rejected,
            }
        }
    }
}

/// Register pack-declared environments and extensions into the live
/// registry, transactionally: either every definition and extension in
/// the call lands and the live registry moves to the returned generation,
/// or nothing changes and the error names the first violation.
///
/// # Errors
///
/// [`EnvironmentRegistrationError`] — a reserved-name claim (with the
/// claiming provenance), an untrusted extension of a compiled base, an
/// unknown extension base, or a §3.3 collision in the rebuilt registry.
pub fn register_environments(
    definitions: Vec<EnvironmentDefinition>,
    extensions: Vec<EnvironmentExtension>,
) -> Result<u64, EnvironmentRegistrationError> {
    let state = STATE.get_or_init(|| Mutex::new(DynamicState::default()));
    let mut state = state.lock().expect("environment registration lock");

    // Candidate dynamic state: committed only when the rebuild succeeds.
    let mut candidate = state.clone();
    for definition in definitions {
        candidate
            .definitions
            .retain(|prior| prior.id != definition.id);
        candidate.definitions.push(definition);
    }
    for extension in extensions {
        if !candidate.extensions.contains(&extension) {
            candidate.extensions.push(extension);
        }
    }

    let generation = current_generation() + 1;
    let registry = assemble(
        &candidate.definitions,
        &candidate.extensions,
        &candidate.sources,
        generation,
    )?;

    *live_cell().lock().expect("live environment registry lock") = Arc::new(registry);
    *state = candidate;
    Ok(generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ingress::{is_known_environment_name, resolve_environment};

    /// The source channel replaces the whole source-keyed state, so two
    /// tests driving it concurrently would retire each other's sources.
    /// Every test that syncs takes this first.
    static SYNCING: Mutex<()> = Mutex::new(());
    use tcl_dialect::model::{
        CoreProfileSelector, DetectionFacts, EnvironmentId, EnvironmentPolicy, Family, KeyedAxis,
        Placement, Release, VersionAxisId, VersionSet, WorldPolicy,
    };

    fn arc(text: &str) -> Arc<str> {
        Arc::from(text)
    }

    fn definition(id: &str, provenance: Provenance) -> EnvironmentDefinition {
        EnvironmentDefinition {
            id: EnvironmentId::new(id),
            aliases: Vec::new(),
            display_name: arc(id),
            editor_identity: None,
            core: Some(CoreProfileSelector {
                family: Family::Tcl,
                default_release: Release::TCL_8_6,
                build: tcl_dialect::model::BuildProfileId::Canonical,
            }),
            targets: VersionSet::from_requirements(VersionAxisId::core(Family::Tcl), &["8.6-"])
                .expect("targets"),
            expected_packages: vec![PackagePlacement {
                package: arc("RegistrationProbe"),
                version: Placement::Keyed(KeyedAxis::ToolVersion),
                ambient: true,
            }],
            policy_defaults: EnvironmentPolicy {
                closed_world: WorldPolicy::Open,
                fixed_ensembles: false,
                strict_ascii: false,
                version_ceiling: None,
            },
            server_detection: DetectionFacts {
                file_extensions: vec![FileExtensionClaim {
                    extension: arc("regprobe"),
                    display_name: arc("Registration Probe"),
                }],
                ..DetectionFacts::default()
            },
            help_terms: Vec::new(),
            provenance,
        }
    }

    /// A registered environment resolves through the one ingress with its
    /// declared facts, at a bumped generation.
    #[test]
    fn a_registered_environment_resolves_with_its_facts() {
        let before = live_environments().generation();
        let generation = register_environments(
            vec![definition("registration-probe-env", Provenance::User)],
            Vec::new(),
        )
        .expect("registration succeeds");
        assert!(generation > before);
        let resolved = resolve_environment("registration-probe-env");
        assert_eq!(resolved.id(), "registration-probe-env");
        assert!(resolved.identity.generation >= generation);
        assert!(
            resolved
                .definition
                .expected_packages
                .iter()
                .any(
                    |placement| placement.package.as_ref() == "RegistrationProbe"
                        && placement.ambient
                )
        );
        assert!(
            resolved
                .definition
                .server_detection
                .file_extensions
                .iter()
                .any(|claim| claim.extension.as_ref() == "regprobe")
        );
    }

    /// D17: a bundled pack restating an environment the compiled seed
    /// already carries from it replaces the seed row rather than colliding
    /// with it; a lower tier claiming the same name is refused with the
    /// provenance named, and the bundled definition keeps resolving.
    #[test]
    fn a_bundled_restatement_replaces_its_seed_row_and_lower_tiers_are_refused() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let seed = tcl_dialect::model::bundled_pack_definitions()
            .into_iter()
            .next()
            .expect("the bundled packs seed at least one environment");
        let id = seed.id.as_str().to_owned();

        let mut restated = seed.clone();
        restated.display_name = arc("Restated From Disk");
        let outcome = sync_environment_sources(vec![EnvironmentSource {
            id: "bundled:restated".to_owned(),
            definitions: vec![restated],
            extensions: Vec::new(),
        }]);
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        assert_eq!(outcome.declared, 1);
        let resolved = resolve_environment(&id);
        assert_eq!(
            resolved.definition.display_name.as_ref(),
            "Restated From Disk"
        );
        assert_eq!(resolved.definition.provenance, Provenance::BundledPack);
        assert_eq!(
            live_environments()
                .definitions()
                .iter()
                .filter(|definition| definition.id == seed.id)
                .count(),
            1,
            "the restatement replaced the seed row"
        );

        let mut hijack = seed.clone();
        hijack.provenance = Provenance::WorkspaceTrusted;
        hijack.display_name = arc("Hijacked");
        let outcome = sync_environment_sources(vec![EnvironmentSource {
            id: "workspace:hijack".to_owned(),
            definitions: vec![hijack],
            extensions: Vec::new(),
        }]);
        assert_eq!(outcome.rejected.len(), 1, "{:?}", outcome.rejected);
        match &outcome.rejected[0].error {
            EnvironmentRegistrationError::Reserved {
                name,
                claimed_by,
                provenance,
            } => {
                assert_eq!(name, &id);
                assert_eq!(claimed_by, &id);
                assert_eq!(*provenance, Provenance::WorkspaceTrusted);
            }
            other => panic!("expected Reserved, got {other:?}"),
        }
        let resolved = resolve_environment(&id);
        assert_eq!(
            resolved.definition.display_name.as_ref(),
            seed.display_name.as_ref()
        );
        assert_eq!(resolved.definition.provenance, Provenance::BundledPack);

        // Leave the source channel as this test found it.
        let _ = sync_environment_sources(Vec::new());
    }

    /// A reserved-name claim fails with the provenance-naming error, and
    /// the live registry is untouched.
    #[test]
    fn a_reserved_name_claim_fails_with_the_provenance_error() {
        let before = live_environments().generation();
        let error = register_environments(
            vec![definition("tcl8.6", Provenance::WorkspaceTrusted)],
            Vec::new(),
        )
        .expect_err("a compiled id is reserved");
        match &error {
            EnvironmentRegistrationError::Reserved {
                name,
                claimed_by,
                provenance,
            } => {
                assert_eq!(name, "tcl8.6");
                assert_eq!(claimed_by, "tcl8.6");
                assert_eq!(*provenance, Provenance::WorkspaceTrusted);
            }
            other => panic!("expected Reserved, got {other:?}"),
        }
        assert!(error.to_string().contains("trusted workspace"));
        // Transactional: the failed call left no generation behind.
        assert!(live_environments().generation() >= before);
        assert_eq!(
            resolve_environment("tcl8.6").definition.provenance,
            Provenance::BuiltIn
        );
    }

    /// A reload's facts reach the `&'static` promotion: the leak map keys
    /// on the registry generation, so a re-registered environment is not
    /// still answered from the pre-reload assembly.
    #[test]
    fn a_reload_refreshes_the_promoted_static_view() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ambient = |ambient: bool| EnvironmentSource {
            id: "sync-reload-pack".to_owned(),
            definitions: vec![EnvironmentDefinition {
                expected_packages: vec![PackagePlacement {
                    package: arc("RegistrationProbe"),
                    version: Placement::Keyed(KeyedAxis::ToolVersion),
                    ambient,
                }],
                ..definition("sync-reload-env", Provenance::User)
            }],
            extensions: Vec::new(),
        };

        let _ = sync_environment_sources(vec![ambient(true)]);
        assert!(
            crate::model::static_document_context_for("sync-reload-env")
                .placement_is_ambient("RegistrationProbe")
        );

        let _ = sync_environment_sources(vec![ambient(false)]);
        assert!(
            !crate::model::static_document_context_for("sync-reload-env")
                .placement_is_ambient("RegistrationProbe"),
            "the promotion served the pre-reload generation"
        );

        let _ = sync_environment_sources(Vec::new());
    }

    /// A pack may extend an environment another pack declares, whichever
    /// order discovery hands the two over in.
    #[test]
    fn a_cross_source_extension_does_not_depend_on_discovery_order() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let extender = EnvironmentSource {
            id: "sync-extender-pack".to_owned(),
            definitions: Vec::new(),
            extensions: vec![EnvironmentExtension {
                base: "sync-extended-env".to_owned(),
                file_extensions: Vec::new(),
                filenames: vec![arc("sync-cross-source.tcl")],
                content_signatures: Vec::new(),
                placements: Vec::new(),
                provenance: Provenance::User,
            }],
        };
        let declarer = EnvironmentSource {
            id: "sync-declarer-pack".to_owned(),
            definitions: vec![definition("sync-extended-env", Provenance::User)],
            extensions: Vec::new(),
        };

        for order in [
            vec![extender.clone(), declarer.clone()],
            vec![declarer, extender],
        ] {
            let ids: Vec<String> = order.iter().map(|source| source.id.clone()).collect();
            let outcome = sync_environment_sources(order);
            assert!(
                outcome.rejected.is_empty(),
                "{ids:?} rejected {:?}",
                outcome.rejected
            );
            assert!(
                live_environments()
                    .resolve("sync-extended-env")
                    .is_some_and(|definition| definition
                        .server_detection
                        .filenames
                        .iter()
                        .any(|name| name.as_ref() == "sync-cross-source.tcl")),
                "{ids:?} lost the extension"
            );
            let _ = sync_environment_sources(Vec::new());
        }
    }

    /// Two independently broken sources are both refused by name, and
    /// neither takes a valid source with it — the case a single-culprit
    /// search cannot resolve, because removing either still leaves the
    /// other breaking the rule.
    #[test]
    fn two_broken_sources_are_isolated_from_the_valid_ones() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let source = |id: &str, environment: &str| EnvironmentSource {
            id: id.to_owned(),
            definitions: vec![definition(environment, Provenance::WorkspaceTrusted)],
            extensions: Vec::new(),
        };
        let outcome = sync_environment_sources(vec![
            source("sync-first-good", "sync-first-good-env"),
            source("sync-first-bad", "tcl8.6"),
            source("sync-second-good", "sync-second-good-env"),
            source("sync-second-bad", "tcl9.0"),
        ]);

        let refused: Vec<&str> = outcome
            .rejected
            .iter()
            .map(|rejected| rejected.source.as_str())
            .collect();
        assert_eq!(refused, ["sync-first-bad", "sync-second-bad"]);
        assert!(
            outcome.rejected.iter().all(|rejected| matches!(
                rejected.error,
                EnvironmentRegistrationError::Reserved { .. }
            )),
            "each source is refused for its own rule: {:?}",
            outcome.rejected
        );
        assert_eq!(outcome.declared, 2);
        assert!(is_known_environment_name("sync-first-good-env"));
        assert!(is_known_environment_name("sync-second-good-env"));
        assert_eq!(
            resolve_environment("tcl8.6").definition.provenance,
            Provenance::BuiltIn
        );

        let _ = sync_environment_sources(Vec::new());
    }

    /// An untrusted extension of a compiled base is refused; a trusted one
    /// lands additively and idempotently.
    #[test]
    fn extensions_respect_the_trust_lattice() {
        let untrusted = EnvironmentExtension {
            base: "expect".to_owned(),
            file_extensions: Vec::new(),
            filenames: vec![arc("registration-probe.exp")],
            content_signatures: Vec::new(),
            placements: Vec::new(),
            provenance: Provenance::WorkspaceUntrusted,
        };
        let error = register_environments(Vec::new(), vec![untrusted])
            .expect_err("untrusted extension of a compiled base");
        assert!(matches!(
            error,
            EnvironmentRegistrationError::UntrustedExtension { .. }
        ));

        let trusted = EnvironmentExtension {
            base: "expect".to_owned(),
            file_extensions: vec![FileExtensionClaim {
                extension: arc("regprobe-exp"),
                display_name: arc("Registration Probe Expect"),
            }],
            filenames: Vec::new(),
            content_signatures: Vec::new(),
            placements: vec![PackagePlacement {
                // The base already places `Expect`; this duplicate claim
                // must be dropped, not stacked.
                package: arc("Expect"),
                version: Placement::TracksBase,
                ambient: true,
            }],
            provenance: Provenance::BundledPack,
        };
        register_environments(Vec::new(), vec![trusted.clone()]).expect("trusted extension");
        register_environments(Vec::new(), vec![trusted]).expect("idempotent re-registration");
        let expect = resolve_environment("expect");
        assert_eq!(
            expect
                .definition
                .server_detection
                .file_extensions
                .iter()
                .filter(|claim| claim.extension.as_ref() == "regprobe-exp")
                .count(),
            1,
            "re-registration must not stack detection rows"
        );
        assert_eq!(
            expect
                .definition
                .expected_packages
                .iter()
                .filter(|placement| placement.package.as_ref() == "Expect")
                .count(),
            1,
            "the base's own placement wins over the extension's duplicate"
        );
        assert!(matches!(
            expect
                .definition
                .expected_packages
                .iter()
                .find(|placement| placement.package.as_ref() == "Expect")
                .map(|placement| &placement.version),
            Some(Placement::Pinned(_))
        ));
    }

    /// An extension of a base nothing declares is refused by name.
    #[test]
    fn an_unknown_extension_base_is_refused() {
        let error = register_environments(
            Vec::new(),
            vec![EnvironmentExtension {
                base: "no-such-environment-anywhere".to_owned(),
                file_extensions: Vec::new(),
                filenames: Vec::new(),
                content_signatures: Vec::new(),
                placements: Vec::new(),
                provenance: Provenance::User,
            }],
        )
        .expect_err("unknown base");
        assert!(matches!(
            error,
            EnvironmentRegistrationError::UnknownExtensionBase { .. }
        ));
    }

    /// A source-keyed sync registers, re-registers idempotently without
    /// moving the generation, and **retires** what a later set drops.
    #[test]
    fn a_synced_source_registers_and_retires() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let source = EnvironmentSource {
            id: "sync-probe-pack".to_owned(),
            definitions: vec![definition("sync-probe-env", Provenance::User)],
            extensions: Vec::new(),
        };
        let outcome = sync_environment_sources(vec![source.clone()]);
        assert!(outcome.changed);
        assert_eq!(outcome.declared, 1);
        assert_eq!(outcome.retired, 0);
        assert!(outcome.rejected.is_empty());
        assert!(is_known_environment_name("sync-probe-env"));
        assert_eq!(resolve_environment("sync-probe-env").id(), "sync-probe-env");

        // The same set again: no rebuild, no generation move.
        let again = sync_environment_sources(vec![source]);
        assert!(!again.changed);
        assert_eq!(again.generation, outcome.generation);

        // The set without it: the environment retires and stops
        // resolving, so a deleted pack cannot outlive its file.
        let empty = sync_environment_sources(Vec::new());
        assert!(empty.changed);
        assert_eq!(empty.retired, 1);
        assert!(!is_known_environment_name("sync-probe-env"));
    }

    /// One rejected source is dropped by id; every other source in the
    /// same sync still registers.
    #[test]
    fn a_rejected_source_does_not_take_the_others_with_it() {
        let _syncing = SYNCING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let good = EnvironmentSource {
            id: "sync-good-pack".to_owned(),
            definitions: vec![definition("sync-good-env", Provenance::User)],
            extensions: Vec::new(),
        };
        let bad = EnvironmentSource {
            id: "sync-bad-pack".to_owned(),
            definitions: vec![definition("tcl8.6", Provenance::WorkspaceTrusted)],
            extensions: Vec::new(),
        };
        let outcome = sync_environment_sources(vec![good, bad]);
        assert!(outcome.changed);
        assert_eq!(outcome.declared, 1);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].source, "sync-bad-pack");
        assert!(matches!(
            outcome.rejected[0].error,
            EnvironmentRegistrationError::Reserved { .. }
        ));
        assert_eq!(resolve_environment("sync-good-env").id(), "sync-good-env");
        assert_eq!(
            resolve_environment("tcl8.6").definition.provenance,
            Provenance::BuiltIn
        );
        let _ = sync_environment_sources(Vec::new());
    }

    /// Registration invalidates through the generation machinery: the
    /// resolved identity's generation moves, so the per-context registry
    /// cache keys a fresh generation.
    #[test]
    fn registration_bumps_the_resolved_generation() {
        let before = resolve_environment("tcl9.0").identity.generation;
        register_environments(
            vec![definition(
                "registration-generation-probe",
                Provenance::User,
            )],
            Vec::new(),
        )
        .expect("registration succeeds");
        let after = resolve_environment("tcl9.0").identity.generation;
        assert!(after > before, "generation must move: {before} -> {after}");
    }
}
