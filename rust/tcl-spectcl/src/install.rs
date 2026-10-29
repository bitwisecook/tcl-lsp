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
//! This is the one rule in `docs/design/registry/spec-packs.md` that decides whether
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
use tcl_registry::registry::CommandRegistry;

use crate::hooks;
use crate::loader::PackCommand;
use crate::pack::{MergedPack, PackSet, installs_over};
use tcl_registry::pack_origin::PackOrigin;
use tcl_registry::security_floor::SecurityFloor;
use tcl_registry::spec::CommandSpec;

/// The registry for `profile` with `packs` installed.
///
/// Cached on `(profile, packs.key)`, so the same pack set resolves to the same
/// registry no matter how many times, or from how many threads, it is asked
/// for. An empty set is not a distinct registry — it is the plain profile
/// registry, which keeps "no packs configured" exactly as cheap as it was
/// before packs existed.
///
/// Either way the registry carries the compiled-in surface of the core family
/// `profile`'s environment runs on (Jim's own commands for a `jim` profile),
/// with a non-empty set's commands indexed over it
/// ([`tcl_registry::model::profile_store_with_overlay`]).
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
    // The compiled-in core surfaces belong to every registry this hands out,
    // whether or not a pack set has been published.
    crate::core_surfaces::ensure();
    if packs.key == 0 || packs.is_empty() {
        return std::sync::Arc::clone(
            tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
        );
    }
    // Resolved BEFORE entering the overlay builder: the builder's closure
    // runs with the registry cache's lock held, and resolving a document
    // context takes that same lock to reach the un-overlaid store — doing
    // it inside the closure deadlocks.
    let context = tcl_registry::model::ingress::static_document_context_for_profile(profile);
    tcl_registry::model::profile_store_with_overlay(profile, packs.key, |registry| {
        install_into(registry, packs, context);
    })
}

/// [`registry_with_packs`] by dialect name, resolved the way every other
/// registry lookup in the server resolves one — so an alias or a typo lands on
/// the same entry it always would, now with the workspace's packs on top.
#[must_use]
pub fn registry_for_dialect_with_packs(dialect: &str, packs: &PackSet) -> Arc<CommandRegistry> {
    registry_with_packs(crate::environment::profile_for_dialect(dialect), packs)
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
/// way in selects the supplied profile's own ambient library set.
///
/// Hosted packages (`Tk`, tcllib, a private pack's own `package require`) are
/// never affected: they are ambient in no profile, so the gate passes them.
fn install_into(
    registry: &mut CommandRegistry,
    packs: &PackSet,
    context: &tcl_registry::model::ResolvedContext,
) {
    let plan = hooks::plan_for(packs);
    install_into_with_plan(registry, packs, context, &plan, None);
}

fn install_into_with_plan(
    registry: &mut CommandRegistry,
    packs: &PackSet,
    context: &tcl_registry::model::ResolvedContext,
    plan: &hooks::HookPlan,
    mut installed_rows: Option<&mut Vec<InstalledHookOrigin>>,
) {
    for pack in &packs.packs {
        // Ambient-package declarations go in unfiltered: they are a claim
        // about the *runtime* this dialect models, not about a command, so
        // there is no spec for the vendor gate below to weigh them against.
        for ambient in &pack.ambient_packages {
            registry.insert_ambient_package(ambient.name, ambient.version);
        }
        // Special variables likewise: each row carries its own dialect
        // gate, which every reader applies at the query.
        for special in &pack.special_vars {
            registry.insert_special_var(special.spec);
        }
        let provenance = pack.provenance();
        for command in &pack.commands {
            // The gates run where a set is assembled (`pack::load_sources`,
            // and the studio's own set): no codegen-axis stamp reaches a
            // registry from a provenance, or a package, whose gate refuses
            // one, and no declaration from a package whose capability holds
            // neither.
            debug_assert!(
                passed_the_gates(command, provenance),
                "`{}` from a {} pack reached an install past a gate it fails",
                command.spec.name,
                tcl_registry::model::provenance_label(provenance),
            );
            if context.required_package_available(command.spec.required_package)
                && installs_over(command, registry)
            {
                let specialised = hooks::specialise(command, plan);
                // Invariant I6. An override replaces the shipped spec
                // wholesale, so without this a four-line workspace pack could
                // strip `exec`'s TAINT_SINK and silence taint diagnostics
                // about the repository that shipped the pack. The floor is not
                // keyed on the tier: see `security_floor`'s module docs.
                let installed = if let Some(shipped) = registry.get(specialised.name) {
                    let mut merged = specialised.clone();
                    SecurityFloor::of(shipped).apply(&mut merged);
                    keep_shipped_compilers(&mut merged, shipped);
                    merged
                } else {
                    let mut added = specialised.clone();
                    added.native_compilation = added
                        .native_compilation
                        .or_else(|| added_command_compiler(&added, registry));
                    added
                };
                let installed: &'static CommandSpec = Box::leak(Box::new(installed));
                registry.insert_static(installed);
                // Which pack it came from: a site specialised on it stamps
                // these facts, and a VM admits the site only while it holds
                // the same ones (`PackSet::fact_stamps`).
                let origin = pack_origin(pack, command);
                if let Some(rows) = installed_rows.as_mut() {
                    rows.push(InstalledHookOrigin {
                        original: command.spec,
                        installed,
                        file: command.file.clone(),
                        pack: pack.name.clone(),
                        provenance,
                        origin: origin.clone(),
                    });
                }
                registry.insert_pack_origin(installed, origin);
                // The body a `-package-source` backing named, as the load read
                // it: the compiler has no file to go to.
                if let Some(text) = &command.reference_text {
                    registry.insert_reference_text(installed, std::sync::Arc::clone(text));
                }
            }
        }
    }
}

/// Registry and source allocations produced together by the actual installer.
/// This retains provenance only; it grants no Native execution permission.
pub(crate) struct OwnedHookRegistry {
    registry: Arc<CommandRegistry>,
    plan: Arc<hooks::HookPlan>,
    profile: &'static DialectProfile,
    installed: Vec<InstalledHookOrigin>,
}

struct InstalledHookOrigin {
    original: &'static CommandSpec,
    installed: &'static CommandSpec,
    file: std::path::PathBuf,
    pack: String,
    provenance: tcl_dialect::model::Provenance,
    origin: PackOrigin,
}

impl OwnedHookRegistry {
    pub(crate) fn registry(&self) -> &Arc<CommandRegistry> {
        &self.registry
    }

    pub(crate) fn plan(&self) -> &Arc<hooks::HookPlan> {
        &self.plan
    }

    pub(crate) fn profile(&self) -> &'static DialectProfile {
        self.profile
    }

    /// Whether this installer published `selected`.
    pub(crate) fn installs(&self, selected: &CommandSpec) -> bool {
        self.installed
            .iter()
            .any(|row| std::ptr::eq(selected, row.installed))
    }

    pub(crate) fn owns_binding(
        &self,
        selected: &CommandSpec,
        original: &CommandSpec,
        file: &std::path::Path,
        pack: &str,
    ) -> bool {
        self.installed
            .iter()
            .filter(|row| {
                std::ptr::eq(selected, row.installed)
                    && std::ptr::eq(original, row.original)
                    && row.file == file
                    && row.pack == pack
                    && hooks::hook_bodies_run(row.provenance)
                    && self.registry.pack_origin(selected) == Some(&row.origin)
            })
            .count()
            == 1
    }
}

/// Build a private reader and capture the exact allocations the installer
/// publishes. The shared overlay cache cannot substitute a same-content load.
pub(crate) fn owned_hook_registry(
    profile: &'static DialectProfile,
    packs: &PackSet,
) -> OwnedHookRegistry {
    crate::core_surfaces::ensure();
    let context = tcl_registry::model::ingress::static_document_context_for_profile(profile);
    let plan = hooks::plan_for(packs);
    let mut installed = Vec::new();
    let registry =
        tcl_registry::model::private_profile_store_with_overlay(profile, packs.key, |registry| {
            install_into_with_plan(registry, packs, context, &plan, Some(&mut installed));
        });
    OwnedHookRegistry {
        registry,
        plan,
        profile,
        installed,
    }
}

/// Distinct private stores one process may build. Each build leaks its
/// installed specs once, so stores are kept by pack key and the count is
/// capped rather than evicted: eviction would not reclaim them.
///
/// Measured 2026-10-28 on `x86_64`: a store leaks one `CommandSpec` of 1824
/// bytes per installed pack command (the two-command fixture leaks 3648),
/// plus a copied subcommand slice only where shipped compilers are
/// re-attached; the store itself is `Arc`-owned and freed. The cap bounds
/// the leak at 64 × 1824 bytes ≈ 114 KiB per installed pack command.
const MAX_OWNED_HOOK_REGISTRIES: usize = 64;

/// Private stores by profile and pack key, built at most once each.
pub(crate) struct OwnedHookRegistryMemo {
    entries: std::sync::Mutex<Vec<(&'static str, u64, Arc<OwnedHookRegistry>)>>,
    cap: usize,
}

impl OwnedHookRegistryMemo {
    pub(crate) const fn new(cap: usize) -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
            cap,
        }
    }

    /// The store for `packs` under `profile`, built on first request.
    /// `None` once the cap is reached for a store not yet built.
    pub(crate) fn get(
        &self,
        profile: &'static DialectProfile,
        packs: &PackSet,
    ) -> Option<Arc<OwnedHookRegistry>> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, _, held)) = entries
            .iter()
            .find(|(name, key, _)| *name == profile.name && *key == packs.key)
        {
            return Some(Arc::clone(held));
        }
        if entries.len() >= self.cap {
            return None;
        }
        let built = Arc::new(owned_hook_registry(profile, packs));
        entries.push((profile.name, packs.key, Arc::clone(&built)));
        Some(built)
    }
}

static OWNED_HOOK_REGISTRIES: OwnedHookRegistryMemo =
    OwnedHookRegistryMemo::new(MAX_OWNED_HOOK_REGISTRIES);

/// The process's bounded private store for `packs` under `profile`.
pub(crate) fn bounded_owned_hook_registry(
    profile: &'static DialectProfile,
    packs: &PackSet,
) -> Option<Arc<OwnedHookRegistry>> {
    OWNED_HOOK_REGISTRIES.get(profile, packs)
}

/// An `-override` describes the shipped command itself: the interpreter's
/// command, and so the compile procedure C registered for it and each of
/// its subcommands, is unchanged by a pack's description of it. A pack never
/// authors a native compiler (the loader refuses that contract); the shipped
/// one is kept wherever the override has none.
fn keep_shipped_compilers(merged: &mut CommandSpec, shipped: &CommandSpec) {
    merged.native_compilation = merged.native_compilation.or(shipped.native_compilation);
    let shipped_compiler = |name: &str| {
        shipped
            .subcommands
            .iter()
            .find(|sub| sub.name == name)
            .and_then(|sub| sub.native_compilation)
    };
    if merged
        .subcommands
        .iter()
        .any(|sub| sub.native_compilation.is_none() && shipped_compiler(sub.name).is_some())
    {
        let subcommands: Vec<_> = merged
            .subcommands
            .iter()
            .map(|sub| {
                let mut sub = sub.clone();
                sub.native_compilation = sub.native_compilation.or(shipped_compiler(sub.name));
                sub
            })
            .collect();
        merged.subcommands = Box::leak(subcommands.into_boxed_slice());
    }
}

/// The compile procedure a fresh interpreter holds for a command a pack adds.
/// A command an extension or a `proc` creates has none (`Tcl_CreateObjCommand`
/// registers no `compileProc`), so it compiles as a generic invocation. An
/// `alias_of` carrying an admitted bundled codegen stamp is specialised as its
/// target (the VM checks the alias hop), so it takes the target's compiler
/// from the registry it is installed into; without the stamp it is an alias
/// command, which no compile procedure compiles either.
fn added_command_compiler(
    added: &CommandSpec,
    registry: &CommandRegistry,
) -> Option<tcl_registry::native_compilation::NativeCompilationSpec> {
    if let Some(target) = added.alias_of
        && crate::stamps::carries_stamp(added)
    {
        return registry
            .get(target)
            .and_then(|target| target.native_compilation);
    }
    Some(tcl_registry::native_compilation::NativeCompilationSpec {
        grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
        operation: tcl_registry::SemanticOperationId::Invoke,
        body: tcl_registry::native_compilation::NativeBodyCompilation::Inherit,
    })
}

/// Whether `command` left the load as its gates would have it: a codegen-axis
/// stamp only from a provenance and a package that admit one, and `alias_of`
/// or a `runtime_backing` only from a tier whose capability holds them.
/// [`install_into`] asserts it.
fn passed_the_gates(command: &PackCommand, provenance: tcl_dialect::model::Provenance) -> bool {
    (crate::stamps::stamps_admitted(provenance, command.dependency_tier)
        || !crate::stamps::carries_stamp(command.spec))
        && crate::stamps::declaration_refusals(command.spec, command.dependency_tier).is_empty()
}

/// The origin `command` of `pack` installs with — the pack's name, the
/// content hash of the source that declared it, and the vocabulary it was
/// read under. [`PackSet::fact_stamps`] builds a VM's held facts from the
/// same answer, so a site's stamp and the facts held for its pack set agree
/// by construction.
pub(crate) fn pack_origin(pack: &MergedPack, command: &PackCommand) -> PackOrigin {
    PackOrigin {
        pack: pack.name.clone(),
        content_hash: command.content_hash,
        vocabulary_version: crate::VOCABULARY_VERSION.to_owned(),
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
            dependency_tier: None,
        }])
    }

    #[test]
    fn private_stores_are_kept_by_pack_key_and_capped() {
        let _cache = cache_guard();
        let dir = tmpdir("private-memo");
        let first = pack_set(
            &dir,
            "first.tclspec",
            "speclib memo_first 1 {command memo_first {const_fold -inputs {words} {words ctx} {fold 1}}}",
        );
        let second = pack_set(
            &dir,
            "second.tclspec",
            "speclib memo_second 1 {command memo_second {const_fold -inputs {words} {words ctx} {fold 2}}}",
        );
        let profile = crate::environment::profile_for_dialect("tcl9.1");
        let memo = OwnedHookRegistryMemo::new(1);
        let held = memo.get(profile, &first).expect("first store");
        assert!(Arc::ptr_eq(&held, &memo.get(profile, &first).unwrap()));
        assert!(
            memo.get(profile, &second).is_none(),
            "the cap refuses a new store"
        );
        assert!(memo.get(profile, &first).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn private_and_cached_stores_expose_the_same_surface() {
        let _cache = cache_guard();
        let dir = tmpdir("private-parity");
        let packs = pack_set(
            &dir,
            "parity.tclspec",
            "speclib parity 1 {\n\
             ambient_package parity_pkg 1.0\n\
             command parity_fold {const_fold -inputs {words} {words ctx} {fold 1}}\n\
             command parity_plain {arity 0..1}\n\
             }",
        );
        for dialect in ["tcl9.1", "tcl8.6"] {
            let profile = crate::environment::profile_for_dialect(dialect);
            let cached = registry_with_packs(profile, &packs);
            let owned = owned_hook_registry(profile, &packs);
            let private = owned.registry();
            assert!(!Arc::ptr_eq(&cached, private), "{dialect}");
            assert_eq!(
                cached.profile().map(|p| p.name),
                private.profile().map(|p| p.name)
            );
            assert_eq!(cached.overlay_generation(), private.overlay_generation());
            assert_eq!(
                cached.ambient_package_rows(),
                private.ambient_package_rows()
            );
            let mut names: Vec<_> = cached.command_names_in_any_dialect().collect();
            let mut private_names: Vec<_> = private.command_names_in_any_dialect().collect();
            names.sort_unstable();
            private_names.sort_unstable();
            assert_eq!(names, private_names, "{dialect}");
            assert!(names.contains(&"parity_fold") && names.contains(&"set"));
            let (query, private_query) = (cached.own_surface_query(), private.own_surface_query());
            for name in &names {
                let ordinary = cached.get_for_surface(name, query);
                let selected = private.get_for_surface(name, private_query);
                assert_eq!(
                    ordinary.map(|spec| format!("{spec:?}")),
                    selected.map(|spec| format!("{spec:?}")),
                    "{dialect} {name}"
                );
                assert_eq!(
                    ordinary.and_then(|spec| cached.pack_origin(spec)),
                    selected.and_then(|spec| private.pack_origin(spec)),
                    "{dialect} {name}"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn private_registry_retains_actual_installed_and_source_allocations() {
        let _cache = cache_guard();
        let dir = tmpdir("private-lineage");
        let packs = pack_set(
            &dir,
            "owner.tclspec",
            "speclib owner 1 {command owned_fold {const_fold -inputs {words} {words ctx} {fold 1}}}",
        );
        let profile = crate::environment::profile_for_dialect("tcl9.1");
        let first = owned_hook_registry(profile, &packs);
        let second = owned_hook_registry(profile, &packs);
        assert!(!first.plan().is_empty());
        assert!(Arc::ptr_eq(first.plan(), second.plan()));
        assert!(!Arc::ptr_eq(first.registry(), second.registry()));
        let row = &first.installed[0];
        let selected = first.registry().get("owned_fold").unwrap();
        let foreign = second.registry().get("owned_fold").unwrap();
        assert!(first.owns_binding(selected, row.original, &row.file, &row.pack));
        assert!(!first.owns_binding(foreign, row.original, &row.file, &row.pack));
        let duplicate_source = Box::leak(Box::new(row.original.clone()));
        assert!(!first.owns_binding(selected, duplicate_source, &row.file, &row.pack));
        assert!(!first.owns_binding(
            selected,
            row.original,
            Path::new("foreign.tclspec"),
            &row.pack
        ));
        assert!(!first.owns_binding(selected, row.original, &row.file, "foreign"));
        assert!(
            std::ptr::fn_addr_eq(selected.const_fold.unwrap(), foreign.const_fold.unwrap()),
            "equal stable thunks do not make installed allocations interchangeable"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_private_store_leaks_one_spec_per_installed_command() {
        let _cache = cache_guard();
        let dir = tmpdir("private-leak");
        let packs = pack_set(
            &dir,
            "leak.tclspec",
            "speclib leak 1 {\n  command leak_a {\n    arity 0\n  }\n  command leak_b {\n    arity 1\n  }\n}\n",
        );
        let profile = crate::environment::profile_for_dialect("tcl9.1");
        let store = owned_hook_registry(profile, &packs);
        assert_eq!(store.installed.len(), 2);
        assert!(
            store
                .installed
                .iter()
                .all(|row| row.installed.subcommands.is_empty())
        );
        let _ = std::fs::remove_dir_all(&dir);
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
        let semantics = registry.effective_semantics();
        assert!(Arc::ptr_eq(&semantics, &registry.effective_semantics()));
        assert!(semantics.binding_names().contains("::mylib::with_var"));
        let facts = semantics
            .command("mylib::with_var")
            .expect("the effective index includes the installed pack command");
        assert_eq!(facts.traits(), spec.traits);
        assert_eq!(facts.lowering_hook(), spec.lowering_hook);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The assertion at the install holds the capability gate as well as the
    /// stamp gate. A command that dodged the load's gate — a transitive
    /// dependency's `alias_of` — fails it, and the same declaration from a
    /// direct dependency, the root or a pack no package ships passes.
    #[test]
    fn the_install_assertion_holds_the_capability_gate_too() {
        use tcl_dialect::model::DependencyTier::{Development, Direct, Root, Transitive};
        let _cache = cache_guard();
        let dir = tmpdir("gates");
        let packs = pack_set(
            &dir,
            "aliased.tclspec",
            "speclib aliased 1 {\n  command aliased::unpack {\n    arity 2..\n    \
             alias_of lassign\n  }\n}\n",
        );
        let pack = &packs.packs[0];
        let mut command = pack.commands[0].clone();
        assert!(command.spec.alias_of.is_some(), "the declaration loaded");
        for (tier, passes) in [
            (None, true),
            (Some(Root), true),
            (Some(Direct), true),
            (Some(Transitive), false),
            (Some(Development), false),
        ] {
            command.dependency_tier = tier;
            assert_eq!(
                passed_the_gates(&command, pack.provenance()),
                passes,
                "{tier:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A pack describes commands; it never authors their compilers. What a
    /// fresh interpreter with the pack loaded compiles each with is derived:
    /// an override is the shipped command, so it keeps the shipped compile
    /// procedure; a command the pack adds (a `proc` or an extension's
    /// `Tcl_CreateObjCommand`) has none, so it is a generic invocation; and an
    /// alias with no admitted stamp is an alias command, which none compiles.
    #[test]
    fn pack_rows_take_the_compilers_a_fresh_interpreter_holds() {
        use tcl_registry::native_compilation::NativeCompilationGrammar;
        let _cache = cache_guard();
        let dir = tmpdir("compilers");
        let packs = pack_set(
            &dir,
            "rows.tclspec",
            "speclib rows 1 {\n  \
             command llength -override {\n    arity 1\n    arg 0 -role Value\n  }\n  \
             command rows::added {\n    arity 1\n    arg 0 -role Value\n  }\n  \
             command rows::unpack {\n    arity 1..\n    alias_of lassign\n  }\n\
             }\n",
        );
        let shipped = tcl_registry::model::static_context_for("tcl8.6").commands();
        let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
        let row = |name: &str| registry.get(name).expect(name).native_compilation;
        assert_eq!(
            row("llength"),
            shipped.get("llength").unwrap().native_compilation,
            "an override keeps the shipped compiler"
        );
        assert!(row("llength").is_some());
        for added in ["rows::added", "rows::unpack"] {
            assert_eq!(
                row(added).map(|spec| spec.grammar),
                Some(NativeCompilationGrammar::NoHook),
                "{added}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An alias whose bundled stamp the tier admits is specialised as its
    /// target, so its row takes the target's compiler from the registry it
    /// is installed into.
    #[test]
    fn an_admitted_alias_stamp_takes_its_targets_compiler() {
        let _cache = cache_guard();
        let specs = tmpdir("alias-compiler").join("specs");
        std::fs::create_dir_all(&specs).expect("specs dir");
        std::fs::write(
            specs.join("vendor.tclspec"),
            "speclib vendor 2.0 {\n  command vendor::unpack {\n    arity 1..\n    \
             arg 0 -role Value\n    arg 1 -role VarWrite\n    alias_of lassign\n    \
             codegen_hook -native Lassign\n  }\n}\n",
        )
        .expect("write pack");
        let set = crate::bundled::load_from(&specs);
        let registry = crate::bundled::registry_for_dialect_from("tcl9.0", &set);
        let unpack = registry.get("vendor::unpack").expect("installed");
        assert!(
            unpack.codegen_hook.is_some(),
            "the bundled stamp is admitted"
        );
        assert_eq!(
            unpack.native_compilation,
            registry.get("lassign").unwrap().native_compilation
        );
        assert!(unpack.native_compilation.is_some());
        let _ = std::fs::remove_dir_all(specs.parent().unwrap());
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
            tcl_registry::model::static_context_for("tcl8.6")
                .commands()
                .as_ref(),
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
        let shipped = tcl_registry::model::static_context_for("tcl8.6")
            .commands()
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
            tcl_registry::model::static_context_for("tcl8.6")
                .commands()
                .ambient_package_floor("Tk"),
            None,
            "the un-overlaid registry is untouched"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Metadata-only ambient-package packs participate in the registry overlay.
    #[test]
    fn a_pack_of_nothing_but_ambient_rows_still_reaches_the_registry() {
        let _cache = cache_guard();
        let dir = tmpdir("ambient-only");
        let packs = pack_set(
            &dir,
            "meta.tclspec",
            "speclib meta 1.2 {\n  ambient_package Tk 8.6\n}\n",
        );
        assert!(
            !packs.is_empty(),
            "a pack declaring an ambient floor is not an empty pack"
        );
        let registry = registry_for_dialect_with_packs("tcl8.6", &packs);
        assert_eq!(
            registry.ambient_package_floor("Tk"),
            Some("8.6"),
            "the floor survives a pack that declares no commands"
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
            tcl_registry::model::static_context_for("tcl8.6")
                .commands()
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
                Some(crate::environment::profile_for_dialect(dialect).name),
                "the profile stamp survives the overlay"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
