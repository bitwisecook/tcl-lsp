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
                "environment `{claimed_by}` claims `{name}`, a compiled environment \
                 name, and it registers from the {} tier; a pack-declared environment \
                 may not claim a reserved compiled id or alias, so nothing from this \
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
        Provenance::WorkspaceUntrusted | Provenance::StudioOverride
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

    // Trust gates before assembly, so the error names the rule rather
    // than a downstream collision.
    let compiled = EnvironmentRegistry::compiled();
    for extension in &candidate.extensions {
        let compiled_base = compiled.resolve(&extension.base);
        if let Some(base) = &compiled_base {
            if untrusted(extension.provenance) {
                return Err(EnvironmentRegistrationError::UntrustedExtension {
                    base: base.id.as_str().to_owned(),
                    provenance: extension.provenance,
                });
            }
        } else if !candidate.definitions.iter().any(|definition| {
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

    // Rebuild: compiled seed + dynamic definitions, then extensions over
    // whichever definition owns the base spelling.
    let mut rebuilt = tcl_dialect::model::compiled_definitions();
    rebuilt.extend(candidate.definitions.iter().cloned());
    for extension in &candidate.extensions {
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

    let generation = live_cell()
        .lock()
        .expect("live environment registry lock")
        .generation()
        + 1;
    let registry = EnvironmentRegistry::new(rebuilt, generation).map_err(|error| match &error {
        EnvironmentRegistryError::ReservedName { name, claimed_by } => {
            let provenance = candidate
                .definitions
                .iter()
                .find(|definition| definition.id.as_str() == claimed_by)
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
    })?;

    *live_cell().lock().expect("live environment registry lock") = Arc::new(registry);
    *state = candidate;
    Ok(generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ingress::resolve_environment;
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
