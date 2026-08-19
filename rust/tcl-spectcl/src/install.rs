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

//! Getting a loaded pack into a live registry — at **workspace scope**.
//!
//! This is the one rule in `docs/design/spec-packs.md` that decides whether
//! packs are fast or slow, and it is a rule about *where*, not *how*:
//!
//! > packs layer into the cached registry at workspace scope, **not** the
//! > per-document overlay path stubs use — a pack is parsed per edit of the
//! > pack, never per edit of the code that uses it.
//!
//! So a pack set is installed into the per-profile cached `CommandRegistry`,
//! keyed by the set's content hash, and every query afterwards — hover, role
//! lookup, arity, taint — takes the identical path a compiled-in spec takes:
//! same map, same struct, same field reads. There is no pack-aware code on any
//! hot path, because after this module there is nothing pack-aware left.
//!
//! ## Collision policy
//!
//! **Shipped wins unless the pack says `-override`.** A pack that redeclares
//! `lsort` gets a notice and the shipped `lsort`; a pack that declares
//! `command lsort -override` gets its own. Both outcomes are reported
//! ([`crate::pack::collision_notices`]) — a silent override is exactly as
//! surprising as a silent drop, and the whole point of the policy is that a
//! private pack cannot quietly change what the standard library means.

use std::sync::Arc;

use tcl_dialect::DialectProfile;
use tcl_registry::profile_queries::ProfileQueries;
use tcl_registry::registry::CommandRegistry;

use crate::hooks;
use crate::pack::{PackSet, installs_over};

/// The registry for `profile` with `packs` installed.
///
/// Cached on `(profile, packs.key)`, so the same pack set resolves to the same
/// registry no matter how many times, or from how many threads, it is asked
/// for. An empty set is not a distinct registry — it is the plain profile
/// registry, which keeps "no packs configured" exactly as cheap as it was
/// before packs existed.
///
/// Returned as an owning handle. This is the pack-carrying axis, so this is
/// exactly the registry that must be *retirable*: hold the handle for as long
/// as you are reading from it and drop it when you are done, and the
/// generation goes away once a later pack edit has superseded it in the cache
/// (`tcl_registry::cache`'s memory note).
#[must_use]
pub fn registry_with_packs(
    profile: &'static DialectProfile,
    packs: &PackSet,
) -> Arc<CommandRegistry> {
    if packs.key == 0 || packs.is_empty() {
        return tcl_registry::registry_handle_for_profile(profile);
    }
    tcl_registry::registry_for_profile_with_overlay(profile, packs.key, |registry| {
        install_into(registry, packs, profile);
    })
}

/// [`registry_with_packs`] by dialect name, resolved the way every other
/// registry lookup in the server resolves one — so an alias or a typo lands on
/// the same entry it always would, now with the workspace's packs on top.
#[must_use]
pub fn registry_for_dialect_with_packs(dialect: &str, packs: &PackSet) -> Arc<CommandRegistry> {
    registry_with_packs(DialectProfile::by_name(dialect), packs)
}

/// Insert every pack command the collision policy admits.
///
/// Insertion order is pack order then declaration order, and
/// `CommandRegistry::get` answers with the *last* spec registered for a name —
/// which is why an `-override` command replaces the shipped one simply by
/// being inserted, with no removal step.
///
/// Each spec goes in **specialised** ([`hooks::specialise`]): a command that
/// declared a `const_fold { … }` body carries that slot's thunk rather than
/// the loader's abstaining placeholder, so the hook answers as soon as a
/// thread has a host ([`hooks::ensure_thread_host`]) and abstains exactly as
/// before until one does. A pack that declares no body is untouched, and pays
/// nothing.
///
/// # The vendor gate
///
/// A command whose `required_package` names a **closed-world vendor** package
/// this profile does not ship ambient is skipped, exactly as
/// [`ProfileQueries::is_available`] would have hidden it afterwards. Skipping
/// it at insert time is not the same as hiding it at query time, and the
/// difference is why this is here: the bundled tier carries *all six* EDA
/// libraries for *every* dialect (discovery cannot know which shell a document
/// is), four of them declare a `report_timing`, and a registry that took the
/// first one would answer a Vivado document with Cadence's spec — which then
/// fails its own package gate and resolves to nothing at all. Filtering on the
/// way in leaves each profile the library set
/// `CommandRegistry::load_eda_packs` used to give it.
///
/// Hosted packages (`Tk`, tcllib, a private pack's own `package require`) are
/// never affected: they are ambient in no profile, so the gate passes them.
fn install_into(registry: &mut CommandRegistry, packs: &PackSet, profile: &'static DialectProfile) {
    let plan = hooks::plan_for(packs);
    for pack in &packs.packs {
        // Ambient-package declarations go in unfiltered: they are a claim
        // about the *runtime* this dialect models, not about a command, so
        // there is no spec for the vendor gate below to weigh them against.
        for ambient in &pack.ambient_packages {
            registry.insert_ambient_package(ambient.name, ambient.version);
        }
        for command in &pack.commands {
            if profile.package_available(command.spec.required_package)
                && installs_over(command, registry)
            {
                registry.insert(hooks::specialise(command, &plan).clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{Origin, PackFile, Tier};
    use std::path::{Path, PathBuf};

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-install-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// Loading a pack writes a compiled-cache entry, and `cache`'s own tests
    /// count the entries under a redirected directory — so every test in this
    /// crate that loads a pack holds the same lock they do.
    fn cache_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::cache::REDIRECT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn pack_set(dir: &Path, name: &str, source: &str) -> PackSet {
        let path = dir.join(name);
        std::fs::write(&path, source).expect("write pack");
        crate::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
        }])
    }

    #[test]
    fn a_pack_command_becomes_a_registry_command() {
        let _cache = cache_guard();
        let dir = tmpdir("install");
        let packs = pack_set(
            &dir,
            "mylib.tclspec",
            "speclib mylib 1 {\n  \
             command mylib::with_var {\n    \
               arity 2..3\n    \
               arg 0 -role VarWrite\n    \
               arg 1 -role Body\n    \
               hover -summary {Run a script with a caller variable bound.}\n  \
             }\n}\n",
        );
        let registry = registry_for_dialect_with_packs("tcl8.6", &packs);

        let spec = registry
            .get("mylib::with_var")
            .expect("the pack command resolves like any other");
        assert_eq!(spec.arity.min, 2);
        assert_eq!(spec.arity.max, 3);
        assert_eq!(
            registry.arg_indices_for_role(
                "mylib::with_var",
                &["counter", "set counter 1"],
                tcl_registry::ArgRole::VarWrite,
            ),
            vec![0],
        );
        assert_eq!(
            registry.arg_indices_for_role(
                "mylib::with_var",
                &["counter", "set counter 1"],
                tcl_registry::ArgRole::Body,
            ),
            vec![1],
        );
        assert!(
            registry.command_names().any(|n| n == "mylib::with_var"),
            "and is enumerable, so completion sees it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_same_pack_set_resolves_to_the_same_registry() {
        let _cache = cache_guard();
        let dir = tmpdir("identity");
        let source = "speclib mylib 1 {\n  command mylib::same { arity 1 }\n}\n";
        let a = pack_set(&dir, "a.tclspec", source);
        let b = pack_set(&dir, "a.tclspec", source);
        assert_eq!(a.key, b.key);
        assert!(Arc::ptr_eq(
            &registry_for_dialect_with_packs("tcl8.6", &a),
            &registry_for_dialect_with_packs("tcl8.6", &b),
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_pack_set_is_the_plain_profile_registry() {
        let _cache = cache_guard();
        let empty = PackSet::default();
        let handle = registry_for_dialect_with_packs("tcl8.6", &empty);
        assert!(std::ptr::eq::<CommandRegistry>(
            handle.as_ref(),
            tcl_registry::registry_for_dialect("tcl8.6"),
        ));
    }

    #[test]
    fn shipped_wins_a_collision_unless_the_pack_says_override() {
        let _cache = cache_guard();
        let dir = tmpdir("collision");
        let polite = pack_set(
            &dir,
            "polite.tclspec",
            "speclib polite 1 {\n  command lsort { arity 99 }\n}\n",
        );
        let registry = registry_for_dialect_with_packs("tcl8.6", &polite);
        let shipped = tcl_registry::registry_for_dialect("tcl8.6")
            .get("lsort")
            .expect("shipped lsort");
        assert_eq!(
            registry.get("lsort").expect("lsort").arity,
            shipped.arity,
            "the shipped spec is untouched"
        );
        let notices = crate::pack::collision_notices(&polite, &registry);
        assert_eq!(notices.len(), 1, "{notices:#?}");
        assert!(notices[0].message.contains("shipped spec wins"));

        let bold = pack_set(
            &dir,
            "bold.tclspec",
            "speclib bold 1 {\n  command lsort -override { arity 99 }\n}\n",
        );
        let overridden = registry_for_dialect_with_packs("tcl8.6", &bold);
        assert_eq!(
            overridden.get("lsort").expect("lsort").arity.min,
            99,
            "`-override` replaces it"
        );
        let notices = crate::pack::collision_notices(&bold, &overridden);
        assert!(notices[0].message.contains("replaces the shipped command"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An `ambient_package` row is not a command, so nothing in the command
    /// install path would carry it. This is the whole journey: pack source →
    /// overlaid registry → the floor the analyser reads.
    #[test]
    fn an_ambient_package_row_reaches_the_overlaid_registry() {
        let _cache = cache_guard();
        let dir = tmpdir("ambient");
        let packs = pack_set(
            &dir,
            "amb.tclspec",
            "speclib amb 1.2 {\n  ambient_package Tk 8.6\n  command demo { arity 1 }\n}\n",
        );
        let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
        assert_eq!(registry.ambient_package_floor("Tk"), Some("8.6"));
        assert_eq!(registry.ambient_package_floor("Itcl"), None);
        assert_eq!(
            tcl_registry::registry_for_dialect("tcl8.6").ambient_package_floor("Tk"),
            None,
            "the un-overlaid registry is untouched"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installing_packs_does_not_disturb_the_plain_registry() {
        let _cache = cache_guard();
        let dir = tmpdir("isolation");
        let packs = pack_set(
            &dir,
            "loud.tclspec",
            "speclib loud 1 {\n  command lsort -override { arity 99 }\n}\n",
        );
        let _ = registry_for_dialect_with_packs("tcl8.6", &packs);
        assert_ne!(
            tcl_registry::registry_for_dialect("tcl8.6")
                .get("lsort")
                .expect("lsort")
                .arity
                .min,
            99,
            "the un-overlaid cache entry is a different registry"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn packs_layer_per_profile_not_globally() {
        let _cache = cache_guard();
        let dir = tmpdir("profiles");
        let packs = pack_set(
            &dir,
            "mylib.tclspec",
            "speclib mylib 1 {\n  command mylib::everywhere { arity 1 }\n}\n",
        );
        for dialect in ["tcl8.6", "tcl9.0", "f5-irules"] {
            let registry = registry_for_dialect_with_packs(dialect, &packs);
            assert!(
                registry.get("mylib::everywhere").is_some(),
                "pack missing from {dialect}"
            );
            assert_eq!(
                registry.profile().map(|p| p.name),
                Some(DialectProfile::by_name(dialect).name),
                "the profile stamp survives the overlay"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
