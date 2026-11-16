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

//! Stamped sites through the tier gate and the VM's admission: alias stamps, changed packs, pack folds and the overlay generation a service compiles against.

use super::*;

/// The tier gate at the emitter: the same pack loaded from the workspace
/// tier keeps `alias_of` but loses the stamp, so the call compiles to
/// generic dispatch — no `lassign` identity at the site and no claim — and
/// the VM runs the module as compiled, the alias answering at run time.
#[test]
fn a_workspace_stamp_is_refused_and_specialises_nothing() {
    let set = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: scratch("workspace").join("vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        unpack_pack("lassign"),
    )]);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
    let unpack = registry.get("vendor::unpack").expect("installed");
    assert_eq!(unpack.alias_of, Some("lassign"));
    assert_eq!(unpack.codegen_hook, None, "the tier gate drops the stamp");

    let module = compile(USE, &registry);
    let bindings = &module.top_level.command_bindings;
    assert!(
        bindings.iter().all(|b| b.identity != "lassign"),
        "{bindings:#?}"
    );
    assert_eq!(
        module.top_level.site_claims,
        [] as [tcl_runtime_api::SiteClaim; 0]
    );

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_on(&mut vm);
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );
    let before = plain.get();
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "1 2");
    assert_eq!(plain.get(), before, "generic dispatch needs no recompile");
}

/// The site records `lassign`'s identity, never the pack command's own name:
/// the binding names the spelling the source wrote and the builtin the
/// stamp is the own of. Beside it the site claims the pack facts that made
/// the target admissible — exactly one of the facts a VM holds for the set.
#[test]
fn an_admitted_alias_stamp_records_the_targets_identity() {
    let (set, registry) = bundled("identity", "lassign");
    assert_eq!(
        registry
            .get("vendor::unpack")
            .expect("installed")
            .codegen_hook,
        Some(tcl_registry::hooks::CodegenHookId::Lassign),
        "the bundled stamp is admitted"
    );
    let module = compile(USE, &registry);
    let bindings = &module.top_level.command_bindings;
    let binding = CommandBindingIdentity::new("vendor::unpack", "lassign");
    assert!(bindings.contains(&binding), "{bindings:#?}");
    assert!(
        bindings.iter().all(|b| b.identity != "vendor::unpack"),
        "{bindings:#?}"
    );

    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].pack, "vendor");
    assert_eq!(held[0].overlay_generation, set.key);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding,
            facts: held[0].clone(),
        }]
    );
}

/// The VM admits the module through the alias hop: `vendor::unpack` is an
/// alias of `lassign`, whose builtin identity is the one recorded, and the
/// VM holds the pack's facts, so the module runs with no plain recompile.
/// Before the alias exists the same module is refused — recompiled plain,
/// where the pack name is unknown.
#[test]
fn the_vm_admits_it_through_the_alias_hop() {
    let (set, registry) = bundled("admitted", "lassign");
    let module = compile(USE, &registry);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_on(&mut vm);
    vm.set_pack_facts(facts(&set));
    let refused = vm.run_module(&module);
    assert_eq!(refused.code, Code::Error, "{}", refused.result.to_str());
    assert!(
        plain.get() > 0,
        "nothing at the pack name: refused, recompiled plain"
    );

    // The namespace first: Tcl 8.4 to 9.1 create it for a qualified alias
    // themselves, and this VM does not yet, so without it the pack name
    // would not resolve at all and the test would prove nothing about the
    // hop.
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(admitted.result.to_str().as_ref(), "1 2");
    assert_eq!(
        plain.get(),
        before,
        "admitted: the specialised module ran as compiled"
    );
}

/// Rung 1's check on a rung-2 site: the VM holds facts for a pack whose
/// content hash differs — the pack changed after the module was compiled —
/// so the site is refused and the module recompiled plain, the alias
/// answering through ordinary dispatch. The negative control: the same VM
/// holding the set's own facts admits it.
#[test]
fn a_changed_pack_invalidates_the_site() {
    let (set, registry) = bundled("changed", "lassign");
    let module = compile(USE, &registry);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_on(&mut vm);
    prepare(
        &mut vm,
        "namespace eval vendor {}\ninterp alias {} vendor::unpack {} lassign",
    );

    let changed: Vec<PackFactStamp> = facts(&set)
        .into_iter()
        .map(|stamp| PackFactStamp {
            content_hash: stamp.content_hash ^ 1,
            ..stamp
        })
        .collect();
    vm.set_pack_facts(changed);
    let before = plain.get();
    let invalidated = vm.run_module(&module);
    assert_eq!(
        invalidated.code,
        Code::Ok,
        "{}",
        invalidated.result.to_str()
    );
    assert_eq!(invalidated.result.to_str().as_ref(), "1 2");
    assert!(
        plain.get() > before,
        "a changed pack: refused, recompiled plain"
    );

    vm.set_pack_facts(facts(&set));
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(plain.get(), before, "the set's own facts admit it");
}

/// Rung 1 on its own: a constant the pack's `const_fold` computed at compile
/// time claims the pack's facts beside the binding, which is the builtin's
/// own. A VM holding no facts refuses the unit though the binding matches —
/// recompiled plain, the builtin answering at run time — and the same VM
/// holding the set's facts runs the folded constant as compiled.
#[test]
fn a_pack_fold_is_admitted_only_under_its_pack_s_facts() {
    let (set, registry) = bundled_source("fold", FOLD_PACK);
    let module = compile("set n [llength {a b c}]\nset n\n", &registry);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert!(
        module
            .top_level
            .command_bindings
            .contains(&CommandBindingIdentity::new("llength", "llength")),
        "{:#?}",
        module.top_level.command_bindings
    );
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_on(&mut vm);
    let refused = vm.run_module(&module);
    assert_eq!(refused.code, Code::Ok, "{}", refused.result.to_str());
    assert_eq!(refused.result.to_str().as_ref(), "3");
    assert!(plain.get() > 0, "no facts held: refused, recompiled plain");

    vm.set_pack_facts(held);
    let before = plain.get();
    let admitted = vm.run_module(&module);
    assert_eq!(admitted.code, Code::Ok, "{}", admitted.result.to_str());
    assert_eq!(admitted.result.to_str().as_ref(), "3");
    assert_eq!(plain.get(), before, "the set's facts admit the fold");
}

/// The seam `tcl-engine-tclvm`'s `with_registry` compiles through: an
/// embedder's registry is an owned value handed to
/// `BytecodeCompileService::new`, and the service compiles for a profile
/// against `project_for_profile`'s view of it. A site resting on a pack
/// there claims the pack as it does on the registry the pack was
/// installed into, and stamps that registry's overlay generation, the number
/// a VM holding `PackSet::fact_stamps` compares. A projection that dropped
/// the pack origins would leave the site unclaimed, and one that dropped the
/// generation would stamp `0`, which no held fact matches. Both rungs: an
/// alias site and a pack's fold over an overridden builtin.
#[test]
fn a_site_compiled_through_the_service_stamps_the_bases_overlay_generation() {
    let profile = tcl_spectcl::environment::profile_for_dialect("tcl9.0");
    let through_the_service = |registry: &CommandRegistry, source: &str| {
        // The cache shares its registries; an owned value to hand the service
        // is the base's projection, which is what the service then projects
        // again for the profile it compiles.
        BytecodeCompileService::new(registry.project_for_profile(profile))
            .compile_for_profile(source, profile)
            .expect("compiles")
    };

    let (set, registry) = bundled("service-alias", "lassign");
    assert_ne!(set.key, 0);
    assert_eq!(registry.overlay_generation(), Some(set.key));
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].overlay_generation, set.key);
    let module = through_the_service(&registry, USE);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding: CommandBindingIdentity::new("vendor::unpack", "lassign"),
            facts: held[0].clone(),
        }],
        "{:#?}",
        module.top_level
    );

    let (set, registry) = bundled_source("service-fold", FOLD_PACK);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    assert_eq!(held[0].overlay_generation, set.key);
    let module = through_the_service(&registry, "set n [llength {a b c}]\nset n\n");
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );
}

/// The seam the workspace's overlay reaches the compiler through: a service
/// built for the profile and the key the packs were installed under compiles
/// against that very generation, so a site resting on a pack claims it and
/// stamps the same overlay generation the owned projection above does — both
/// rungs, an alias site and a pack's fold over an overridden builtin. A key
/// nothing installed builds no service, and a service does not compile once
/// its generation is not installed for the profile it is asked for: it does
/// not fall back to the registry the pack is missing from.
#[test]
fn a_service_for_the_packs_overlay_compiles_against_the_generation_they_installed() {
    let profile = tcl_spectcl::environment::profile_for_dialect("tcl9.0");
    let through_the_overlay = |set: &PackSet, source: &str| {
        BytecodeCompileService::for_profile_with_overlay(profile, set.key)
            .expect("the packs' overlay is installed")
            .compile_for_profile(source, profile)
            .expect("compiles")
    };

    let (set, _registry) = bundled("overlay-alias", "lassign");
    assert_ne!(set.key, 0);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    let module = through_the_overlay(&set, USE);
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::BuiltinAlias {
            binding: CommandBindingIdentity::new("vendor::unpack", "lassign"),
            facts: held[0].clone(),
        }],
        "{:#?}",
        module.top_level
    );

    let (set, _registry) = bundled_source("overlay-fold", FOLD_PACK);
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");
    let module = through_the_overlay(&set, "set n [llength {a b c}]\nset n\n");
    assert_eq!(
        module.top_level.site_claims,
        vec![SiteClaim::PackFacts(held[0].clone())],
        "{:#?}",
        module.top_level
    );

    // Nothing installed this key: no service, and the miss names it.
    let missing = 0x0BAD_0BAD;
    let miss = BytecodeCompileService::for_profile_with_overlay(profile, missing)
        .err()
        .expect("nothing installed the overlay");
    assert_eq!(miss.overlay, missing);
    assert_eq!(miss.environment, "tcl9.0");

    // The pack is installed for `tcl9.0` and not for `tcl8.6`, so the service
    // does not compile for 8.6 either, rather than compile it plain.
    let service = BytecodeCompileService::for_profile_with_overlay(profile, set.key)
        .expect("installed for 9.0");
    let older = tcl_spectcl::environment::profile_for_dialect("tcl8.6");
    let refused = service
        .compile_for_profile(USE, older)
        .expect_err("no generation for 8.6 under this overlay");
    let refused_text = refused.to_string();
    assert!(
        refused_text.contains("tcl8.6") && refused_text.contains("not installed"),
        "{refused:?}"
    );
}

/// The negative: a proc at the pack name is not the builtin, so the VM
/// refuses the specialised site and recompiles the module plain — the proc
/// runs, where the specialised `lassign` code would have assigned `1 2` —
/// even though it holds the pack's facts.
#[test]
fn a_proc_at_the_pack_name_recompiles_plain() {
    let (set, registry) = bundled("proc", "lassign");
    let module = compile(USE, &registry);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_on(&mut vm);
    vm.set_pack_facts(facts(&set));
    prepare(
        &mut vm,
        "namespace eval vendor {}\n\
         proc vendor::unpack {l a b} {upvar 1 $a x $b y; set x P; set y Q}",
    );
    let before = plain.get();
    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "P Q");
    assert!(plain.get() > before, "refused: recompiled plain");
}
