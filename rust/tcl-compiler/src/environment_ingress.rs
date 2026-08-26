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

//! The compiler's dialect-name ingress on the centralised environment
//! model (centralisation contract R-a, ledger row C2 — the compiler
//! half of P1-F).
//!
//! Every place this crate accepts a dialect *name* resolves it here,
//! **once**, to an [`EnvironmentDefinition`] through the one
//! [`EnvironmentRegistry`] resolver, and derives everything downstream
//! from the resolved environment: the per-context registry generation
//! ([`ContextRegistry`], carrying the [`ResolvedContext`] availability
//! view and the generation's command store), the Tk-environment fact
//! that used to ride `availability_for_name`'s `TK`-bit union, and — as
//! the documented wave-1 interop — the old interned [`DialectProfile`]
//! the rest of the pipeline still threads for grammar, versions, and
//! diagnostic labels. The interop mapping goes through the **resolved
//! canonical id**, never back through the raw string, so the retired
//! name validators (`by_name` at ingress, `resolve_known`,
//! `availability_for_name`, raw `DialectSet::parse`) have no remaining
//! caller in this crate outside the projections documented at their
//! sites.
//!
//! [`ResolvedContext`]: tcl_registry::model::ResolvedContext

use std::sync::{Arc, Mutex, OnceLock};

use tcl_dialect::model::{EnvironmentDefinition, EnvironmentIdentity, EnvironmentRegistry};
use tcl_dialect::{DialectProfile, LibraryVersionOverrides};
use tcl_registry::model::{ContextRegistry, KeyedVersions, registry_for_environment_if_built};

/// The compiled environment registry, resolved once per process. Pack- or
/// configuration-declared environments join here when the compiler gains
/// a dynamic-environment channel (P2); until then generation 0 is the
/// whole world, matching the compiled catalogue the old ingress read.
fn environments() -> &'static EnvironmentRegistry {
    static CELL: OnceLock<EnvironmentRegistry> = OnceLock::new();
    CELL.get_or_init(EnvironmentRegistry::compiled)
}

/// One resolved document environment: the definition plus the identity
/// its registry generations key on.
#[derive(Debug, Clone)]
pub(crate) struct DocumentEnvironment {
    /// The resolved definition.
    pub definition: Arc<EnvironmentDefinition>,
    /// The `(id, generation, overlay)` identity for generation caching.
    pub identity: EnvironmentIdentity,
}

/// Resolve a user-written dialect name — canonical id, alias, or editor
/// language id — to its environment. Unknown names (and the empty
/// string) resolve to the lenient `tcl` environment, exactly as every
/// unknown name resolved to the permissive fallback profile before.
pub(crate) fn resolve_environment(name: &str) -> DocumentEnvironment {
    let registry = environments();
    let definition = registry.resolve(name).unwrap_or_else(|| {
        registry
            .resolve("tcl")
            .expect("the compiled catalogue seeds the lenient `tcl` environment")
    });
    let identity = registry.identity_of(&definition);
    DocumentEnvironment {
        definition,
        identity,
    }
}

impl DocumentEnvironment {
    /// Whether this is the `tk` environment — the fact that used to ride
    /// `availability_for_name`'s `TK`-bit union (`tk` is the one ingress
    /// whose surface is a library placement rather than a profile). The
    /// placement-driven form (`ResolvedContext::package_active("Tk")`)
    /// takes over in P3, when the Tk pilot makes the placement ambient
    /// under `wish` and W120's require-nag semantics move with it.
    #[must_use]
    pub(crate) fn is_tk(&self) -> bool {
        self.definition.id.as_str() == "tk"
    }

    /// The interned profile the analyser threads for this environment —
    /// wave-1 interop (deleted in P1-G): the catalogue environments map
    /// to their same-named profile; the model-only ids (`tcl`, `tk`)
    /// map to the permissive fallback, exactly as the old name ingress
    /// resolved them.
    #[must_use]
    pub(crate) fn analyser_profile(&self) -> &'static DialectProfile {
        DialectProfile::find(self.definition.id.as_str()).unwrap_or_else(DialectProfile::plain_tcl)
    }

    /// The interned profile a compilation unit is built for — as
    /// [`Self::analyser_profile`], except the `tk` environment keeps its
    /// typed additive ingress profile (the old `resolve_known`
    /// promotion), so the unit's semantic bit and availability mask
    /// carry the Tk surface.
    #[must_use]
    pub(crate) fn unit_profile(&self) -> &'static DialectProfile {
        if self.is_tk() {
            DialectProfile::tk()
        } else {
            self.analyser_profile()
        }
    }

    /// The keyed-axis pins for this session's library-version overrides.
    /// A malformed override spelling drops to the axis default rather
    /// than aborting resolution (the old string-floor path carried the
    /// spelling verbatim; a non-version spelling never named a real
    /// floor).
    #[must_use]
    pub(crate) fn keyed_versions(overrides: &LibraryVersionOverrides) -> KeyedVersions {
        KeyedVersions::from_overrides(overrides).unwrap_or_default()
    }

    /// The registry generation for this environment at `overlay` — the
    /// pack-overlay key threaded exactly as the old
    /// `registry_for_profile_if_built(profile, overlay)` door: a
    /// not-yet-installed overlay falls back to the un-overlaid
    /// generation, the state the process was in a moment ago.
    #[must_use]
    pub(crate) fn context_registry(
        &self,
        keyed: &KeyedVersions,
        overlay: u64,
    ) -> Arc<ContextRegistry> {
        registry_for_environment_if_built(&self.definition, &self.identity, keyed, overlay)
            .unwrap_or_else(|| {
                registry_for_environment_if_built(&self.definition, &self.identity, keyed, 0)
                    .expect("the un-overlaid generation always builds")
            })
    }
}

/// The generation a **profile-keyed** consumer reads (default keyed
/// axes, no pack overlay) — transitional plumbing for passes that still
/// receive a resolved [`DialectProfile`] instead of a dialect name
/// (side-effect classification, the fixed iRules handles). The profile's
/// canonical name is a canonical environment id, so this is an id-keyed
/// lookup, not a re-parse.
#[must_use]
pub(crate) fn context_for_profile(profile: &DialectProfile) -> Arc<ContextRegistry> {
    resolve_environment(profile.name).context_registry(&KeyedVersions::default(), 0)
}

/// The fixed iRules generation — the environment-model face of the old
/// `DialectProfile::irules()` handle for the hardcoded iRules lookups.
#[must_use]
pub(crate) fn irules_context() -> Arc<ContextRegistry> {
    resolve_environment("f5-irules").context_registry(&KeyedVersions::default(), 0)
}

/// Intern `name` as a `&'static str` — transitional plumbing for the
/// version-gate axis, whose `Package` arm predates the model's
/// `Arc<str>` package names. Bounded by the compiled placement
/// vocabulary (each distinct name leaks once). Retired with the axis's
/// re-typing in P1-G.
#[must_use]
pub(crate) fn interned_package_name(name: &str) -> &'static str {
    static CELL: OnceLock<Mutex<Vec<&'static str>>> = OnceLock::new();
    let interned = CELL.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = interned.lock().expect("package-name intern mutex");
    if let Some(&existing) = guard.iter().find(|&&existing| existing == name) {
        return existing;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    guard.push(leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve_as_the_old_ingress_did() {
        // Catalogue names and aliases → their same-named profile.
        for (name, profile) in [
            ("tcl8.6", "tcl8.6"),
            ("f5-irules", "f5-irules"),
            ("irules", "f5-irules"),
            ("tcl-irule", "f5-irules"),
            ("f5-iapps", "f5-iapps"),
            ("xilinx-eda-tcl", "xilinx-eda-tcl"),
        ] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), profile, "{name}");
            assert_eq!(environment.analyser_profile().name, profile, "{name}");
            assert_eq!(environment.unit_profile().name, profile, "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        // Unknown names and the bare `tcl` land on the lenient
        // environment and the permissive fallback profile.
        for name in ["", "tcl", "no-such-dialect"] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), "tcl", "{name}");
            assert!(environment.analyser_profile().is_fallback(), "{name}");
            assert!(environment.unit_profile().is_fallback(), "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        // The `tk` ingress: permissive analyser profile (the old
        // `by_name` answer), typed additive unit profile (the old
        // `resolve_known` answer), and the Tk-environment fact set.
        let tk = resolve_environment("tk");
        assert!(tk.is_tk());
        assert!(tk.analyser_profile().is_fallback());
        assert_eq!(tk.unit_profile().name, "tk");
    }

    #[test]
    fn context_registries_carry_the_expected_stores() {
        let environment = resolve_environment("tcl8.5");
        let generation = environment.context_registry(&KeyedVersions::default(), 0);
        assert_eq!(
            generation.context().environment.id.as_str(),
            "tcl8.5",
            "the generation answers under the resolved environment"
        );
        // An uninstalled pack overlay falls back to the un-overlaid
        // generation, exactly as the old if-built door did.
        let fallback = environment.context_registry(&KeyedVersions::default(), 0xDEAD);
        assert!(Arc::ptr_eq(generation.commands(), fallback.commands()));
    }

    #[test]
    fn package_names_intern_stably() {
        let first = interned_package_name("f5-irules-cmds");
        let second = interned_package_name("f5-irules-cmds");
        assert!(std::ptr::eq(first, second));
        assert_eq!(first, "f5-irules-cmds");
    }
}
