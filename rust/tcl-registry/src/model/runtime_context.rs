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

//! The runtime's counterpart of the analysis context: the world a runtime is
//! pinned to, resolved through the same ingress the compiler uses.
//!
//! A [`RuntimeContext`] is data — environment, release point, build, package
//! floors, overlay generation — and this module is where it meets the
//! registry. [`runtime_context_of`] and [`runtime_context_for_profile`] state
//! the context an environment or a profile resolves to, which is what a
//! compile records in an artefact's manifest and what a runtime pinned the
//! same way holds. [`pin`] goes the other way: it resolves a context to the
//! profile a runtime installs and the generation it holds, and refuses a
//! context the ingress does not agree with, an overlay nothing has installed
//! included — an overlay miss is an error and never the un-overlaid generation
//! under another name.

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use rustc_hash::FxHashMap;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::{BuildProfileId, DialectPoint};
use tcl_runtime_api::{ArtefactIdentityManifest, PackFactStamp, RuntimeContext};

use crate::intrinsic::intrinsic_table_hash;
use crate::model::assembly::{ContextRegistry, OverlayMiss};
use crate::model::context::KeyedVersions;
use crate::model::ingress::{
    DocumentEnvironment, environments, resolve_environment, resolve_known_environment,
};

/// The context `environment` resolves to at its own point, under the pack
/// overlay `overlay_generation`: its canonical id, the release and build of
/// its point (empty and unmeasured for an environment with no Tcl ladder), and
/// the package floors its un-overlaid generation establishes.
#[must_use]
pub fn runtime_context_of(
    environment: &DocumentEnvironment,
    overlay_generation: u64,
) -> RuntimeContext {
    let point = environment.point();
    RuntimeContext {
        environment: environment.id().to_owned(),
        release: point.map_or_else(String::new, |point| point.release().as_str().to_owned()),
        build: point.map_or(BuildProfileId::Unknown, DialectPoint::build),
        packages: environment
            .default_context_registry()
            .context()
            .package_floors(),
        overlay_generation,
    }
}

/// The context the environment `profile` names resolves to, with no overlay:
/// the profile form of a pin. A profile's canonical name is an environment
/// id, and one no environment answers to sinks to the lenient `tcl`
/// environment, as it does everywhere else.
///
/// Memoised per profile name and environment-registry generation: a compile
/// states it once per module, and resolving an environment costs tens of
/// microseconds.
#[must_use]
pub fn runtime_context_for_profile(profile: &DialectProfile) -> RuntimeContext {
    type Memo = Mutex<FxHashMap<(&'static str, u64), RuntimeContext>>;
    static MEMO: OnceLock<Memo> = OnceLock::new();
    let key = (profile.name, environments().generation());
    let memo = MEMO.get_or_init(Memo::default);
    if let Some(hit) = memo.lock().expect("runtime context memo").get(&key) {
        return hit.clone();
    }
    let context = runtime_context_of(&resolve_environment(profile.name), 0);
    memo.lock()
        .expect("runtime context memo")
        .insert(key, context.clone());
    context
}

/// A context resolved through the ingress: what a runtime installs to pin
/// itself, and the identity that pin states.
#[derive(Clone)]
pub struct PinnedContext {
    /// The context that was pinned.
    pub context: RuntimeContext,
    /// The interned profile of the resolved environment.
    pub profile: &'static DialectProfile,
    /// The registry generation at the context's overlay, held for as long as
    /// the pin stands, so that it stays available to the runtime however the
    /// process cache retires overlays (past 64 entries). `None`
    /// for the profile form, whose un-overlaid generation is retained for
    /// good.
    pub generation: Option<Arc<ContextRegistry>>,
    identity: ArtefactIdentityManifest,
}

impl PinnedContext {
    /// The profile form of a pin: the context `profile` names, with no
    /// overlay.
    #[must_use]
    pub fn for_profile(profile: &'static DialectProfile) -> Self {
        Self::new(runtime_context_for_profile(profile), profile, None)
    }

    fn new(
        context: RuntimeContext,
        profile: &'static DialectProfile,
        generation: Option<Arc<ContextRegistry>>,
    ) -> Self {
        let identity = context.identity(&[], intrinsic_table_hash());
        Self {
            context,
            profile,
            generation,
            identity,
        }
    }

    /// Restate the identity with the pack facts the runtime holds: the pin's
    /// context, those facts, this build's intrinsic table, ABI and embedded
    /// library. What a compiled artefact's manifest is compared with.
    pub fn restate(&mut self, packs: &[PackFactStamp]) {
        self.identity = self.context.identity(packs, intrinsic_table_hash());
    }

    /// The identity this pin states, in the shape an artefact states its own.
    #[must_use]
    pub fn identity(&self) -> &ArtefactIdentityManifest {
        &self.identity
    }
}

impl fmt::Debug for PinnedContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PinnedContext")
            .field("context", &self.context)
            .field("profile", &self.profile.name)
            .finish_non_exhaustive()
    }
}

/// Why a [`RuntimeContext`] was not pinned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinError {
    /// No environment answers to the name.
    UnknownEnvironment(String),
    /// The release is not the one the environment's point is at.
    ReleaseDisagrees {
        /// The environment's canonical id.
        environment: String,
        /// The release the context named.
        named: String,
        /// The release the environment's point is at.
        resolved: String,
    },
    /// The build is not the one the environment's point is at.
    BuildDisagrees {
        /// The environment's canonical id.
        environment: String,
        /// The build the context named.
        named: BuildProfileId,
        /// The build the environment's point is at.
        resolved: BuildProfileId,
    },
    /// The overlay the context names is not installed for the environment.
    OverlayMiss(OverlayMiss),
}

impl fmt::Display for PinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEnvironment(name) => write!(f, "no environment answers to `{name}`"),
            Self::ReleaseDisagrees {
                environment,
                named,
                resolved,
            } => write!(
                f,
                "`{environment}` is at release `{resolved}`, not `{named}`"
            ),
            Self::BuildDisagrees {
                environment,
                named,
                resolved,
            } => write!(f, "`{environment}` is at build {resolved:?}, not {named:?}"),
            Self::OverlayMiss(miss) => miss.fmt(f),
        }
    }
}

impl std::error::Error for PinError {}

impl From<OverlayMiss> for PinError {
    fn from(miss: OverlayMiss) -> Self {
        Self::OverlayMiss(miss)
    }
}

/// Resolve `context` through the ingress.
///
/// The environment must exist, the release and build must be those its point
/// is at, and the overlay must be installed (`0` is none and always is). The
/// package floors are the host's own statement and are carried as given.
///
/// # Errors
///
/// [`PinError`] naming what the ingress did not agree with.
pub fn pin(context: &RuntimeContext) -> Result<PinnedContext, PinError> {
    let environment = resolve_known_environment(&context.environment)
        .ok_or_else(|| PinError::UnknownEnvironment(context.environment.clone()))?;
    let resolved = runtime_context_of(&environment, context.overlay_generation);
    if resolved.release != context.release {
        return Err(PinError::ReleaseDisagrees {
            environment: resolved.environment,
            named: context.release.clone(),
            resolved: resolved.release,
        });
    }
    if resolved.build != context.build {
        return Err(PinError::BuildDisagrees {
            environment: resolved.environment,
            named: context.build,
            resolved: resolved.build,
        });
    }
    let generation =
        environment.context_registry(&KeyedVersions::default(), context.overlay_generation)?;
    Ok(PinnedContext::new(
        context.clone(),
        environment.unit_profile(),
        Some(generation),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_profile() -> Vec<&'static DialectProfile> {
        DialectProfile::all()
            .iter()
            .chain([DialectProfile::plain_tcl(), DialectProfile::tk()])
            .collect()
    }

    #[test]
    fn a_profiles_context_is_its_environments_own_point() {
        for profile in every_profile() {
            let environment = resolve_environment(profile.name);
            let context = runtime_context_for_profile(profile);
            assert_eq!(context.environment, environment.id(), "{}", profile.name);
            assert_eq!(context.overlay_generation, 0);
            let point = environment.point();
            assert_eq!(
                context.release,
                point.map_or("", |point| point.release().as_str()),
                "{}",
                profile.name
            );
            assert_eq!(
                context.build,
                point.map_or(BuildProfileId::Unknown, DialectPoint::build),
                "{}",
                profile.name
            );
        }
        let tcl86 = runtime_context_for_profile(DialectProfile::find("tcl8.6").unwrap());
        assert_eq!(
            (tcl86.environment.as_str(), tcl86.release.as_str()),
            ("tcl8.6", "8.6")
        );
        assert_eq!(tcl86.build, BuildProfileId::Canonical);
    }

    #[test]
    fn a_profiles_context_pins_back_to_that_profile() {
        for profile in every_profile() {
            let pinned = pin(&runtime_context_for_profile(profile)).unwrap_or_else(|error| {
                panic!("{} did not pin: {error}", profile.name);
            });
            assert!(
                std::ptr::eq(pinned.profile, profile),
                "{} pinned as {}",
                profile.name,
                pinned.profile.name
            );
        }
    }

    #[test]
    fn a_context_the_ingress_disagrees_with_is_not_pinned() {
        let context = runtime_context_for_profile(DialectProfile::find("tcl8.6").unwrap());
        let edit = |edit: fn(&mut RuntimeContext)| {
            let mut context = context.clone();
            edit(&mut context);
            pin(&context).expect_err("the ingress disagrees")
        };
        assert_eq!(
            edit(|c| c.environment = "no-such-environment".to_owned()),
            PinError::UnknownEnvironment("no-such-environment".to_owned())
        );
        assert_eq!(
            edit(|c| c.release = "9.0".to_owned()),
            PinError::ReleaseDisagrees {
                environment: "tcl8.6".to_owned(),
                named: "9.0".to_owned(),
                resolved: "8.6".to_owned(),
            }
        );
        assert_eq!(
            edit(|c| c.build = BuildProfileId::JimFull),
            PinError::BuildDisagrees {
                environment: "tcl8.6".to_owned(),
                named: BuildProfileId::JimFull,
                resolved: BuildProfileId::Canonical,
            }
        );
    }

    #[test]
    fn an_overlay_nothing_installed_is_an_error_and_not_the_plain_generation() {
        const OVERLAY: u64 = 0x0C0_1701;
        let profile = DialectProfile::find("tcl9.0").unwrap();
        let mut context = runtime_context_for_profile(profile);
        context.overlay_generation = OVERLAY;
        assert_eq!(
            pin(&context).expect_err("nothing installed it"),
            PinError::OverlayMiss(OverlayMiss {
                environment: "tcl9.0".to_owned(),
                overlay: OVERLAY,
            })
        );
        context.overlay_generation = 0;
        assert!(pin(&context).is_ok(), "no overlay never misses");
    }

    #[test]
    fn an_installed_overlay_is_pinned_and_held() {
        const OVERLAY: u64 = 0x0C0_1702;
        let profile = DialectProfile::find("tcl9.0").unwrap();
        let installed = crate::registry_for_profile_with_overlay(profile, OVERLAY, |registry| {
            let mut custom = registry.get("llength").expect("llength spec").clone();
            custom.name = "overlay::length";
            custom.surface = None;
            registry.insert(custom);
        });
        let mut context = runtime_context_for_profile(profile);
        context.overlay_generation = OVERLAY;
        let pinned = pin(&context).expect("installed, so it pins");
        let held = pinned
            .generation
            .as_ref()
            .expect("a context pin holds its generation");
        assert!(Arc::ptr_eq(held.commands(), &installed));
        assert!(held.commands().get("overlay::length").is_some());
        assert!(std::ptr::eq(pinned.profile, profile));
        assert_eq!(pinned.context, context);
        assert!(
            PinnedContext::for_profile(profile).generation.is_none(),
            "the profile form holds no overlay"
        );
    }

    #[test]
    fn a_pin_states_its_context_and_the_facts_it_holds_as_an_artefact_would() {
        let profile = DialectProfile::find("tcl8.6").unwrap();
        let mut pin = PinnedContext::for_profile(profile);
        let bare = pin.context.identity(&[], intrinsic_table_hash());
        assert_eq!(pin.identity(), &bare);
        assert!(pin.identity().packs.is_empty());

        let stamp = PackFactStamp {
            pack: "vendor".to_owned(),
            content_hash: 5,
            vocabulary_version: "2".to_owned(),
            overlay_generation: 9,
            evaluator_revision: 0,
        };
        pin.restate(&[stamp.clone(), stamp.clone()]);
        assert_eq!(pin.identity().packs, vec![stamp]);
        assert_eq!(pin.identity().environment, "tcl8.6");
        assert_eq!(pin.identity().intrinsic_table_hash, intrinsic_table_hash());

        pin.restate(&[]);
        assert_eq!(pin.identity(), &bare);
    }

    #[test]
    fn the_package_floors_a_context_carries_are_the_hosts_statement() {
        let profile = DialectProfile::find("tcl8.6").unwrap();
        let mut context = runtime_context_for_profile(profile);
        context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
        let pinned = pin(&context).expect("floors are not checked against the environment");
        assert_eq!(pinned.context.packages, context.packages);
    }
}
