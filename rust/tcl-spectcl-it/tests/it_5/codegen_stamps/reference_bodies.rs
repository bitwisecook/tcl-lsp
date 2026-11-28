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

//! Commands a pack backs with a Tcl body: inlined and admitted, host-native backings, bodies that diverge and bodies read from a package source.

use super::*;

/// Rung 3: a procedure calling a command a pack declares as a Tcl body has
/// the pack's definition inlined, with the procedure binding that holds the
/// live command to it and the claim on the pack that declared the backing.
/// The VM holding that pack's facts, with a live procedure of exactly the
/// claimed text, runs the inlined function as compiled — no plain recompile.
#[test]
fn a_tcl_body_backed_command_is_inlined_and_admitted() {
    crate::install_surfaces();
    let (set, _registry) =
        bundled_source("reference-admitted", &reference_pack(DOUBLE_IN_THE_PACK));
    let held = facts(&set);
    assert_eq!(held.len(), 1, "one pack file: {held:#?}");

    let module = service_for(&set)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = fixture_procedure(&module, "::caller");
    assert_eq!(caller.procedure_bindings, vec![double_binding()]);
    assert_eq!(
        caller.site_claims,
        vec![SiteClaim::ReferenceBody {
            procedure: double_binding(),
            backing: BackingKind::TclBody,
            facts: held[0].clone(),
        }]
    );
    assert!(!calls_a_command(caller), "{:#?}", caller.instructions);
    assert!(
        caller
            .instructions
            .iter()
            .any(|instruction| instruction.source_cmd_text == "expr {$x * 2}"),
        "the inlined commands keep the text of the definition they came from: {:#?}",
        caller.instructions
    );
    assert_eq!(
        module.source, CALLER,
        "the artefact's source is the module's own"
    );
    assert_eq!(module.claimed_packs(), held);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(held.clone());
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(
        plain.get(),
        0,
        "admitted: the inlined function ran as compiled"
    );

    // The same VM without the pack's facts refuses the claim: the call still
    // answers, through the live procedure, after a plain recompile.
    vm.set_pack_facts(Vec::new());
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert!(plain.get() > 0, "no facts held: refused, recompiled plain");
}

/// The negative: the same definition, from a command the pack backs with the
/// host, is neither inlined nor activated. Compiled against that pack nothing
/// is inlined, no binding is recorded and the call stays a call. And a unit
/// that claims the body for such a command — here the inlining pack's own
/// compile, restated as host-native — is refused though the live procedure's
/// text matches exactly, because an exact match says nothing about whether the
/// command *is* that procedure.
#[test]
fn a_host_native_backing_never_defines_a_proc() {
    crate::install_surfaces();
    let (native, _registry) = bundled_source("reference-native", &reference_pack("host-native"));
    let module = service_for(&native)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = fixture_procedure(&module, "::caller");
    assert_eq!(
        caller.procedure_bindings,
        [] as [tcl_runtime_api::ProcedureBindingIdentity; 0]
    );
    assert_eq!(caller.site_claims, [] as [tcl_runtime_api::SiteClaim; 0]);
    assert!(calls_a_command(caller), "{:#?}", caller.instructions);

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&native), None);
    vm.set_pack_facts(facts(&native));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(
        plain.get(),
        0,
        "generic dispatch to the live command needs no recompile"
    );

    // A compiler that claimed the body for that command anyway.
    let (inlining, _registry) =
        bundled_source("reference-forged", &reference_pack(DOUBLE_IN_THE_PACK));
    for (forged, refused) in [
        (None, false),
        (Some(Forgery::Backing(BackingKind::HostNative)), true),
        (Some(Forgery::Backing(BackingKind::None)), true),
        (Some(Forgery::Backing(BackingKind::ShippedBuiltin)), true),
        (Some(Forgery::Backing(BackingKind::TclBody)), false),
        // The backing is right and the claim names a procedure the function
        // does not hold the live command to.
        (Some(Forgery::Procedure), true),
    ] {
        let mut vm = vm_for_overlay();
        let plain = PlainCounting::installed_with(&mut vm, service_for(&inlining), forged);
        vm.set_pack_facts(facts(&inlining));
        define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
        assert_eq!(call_caller(&mut vm, "21"), "42", "{forged:?}");
        assert_eq!(plain.get() > 0, refused, "{forged:?}");
    }
}

/// A body the pack carries is text that goes stale without anyone touching the
/// pack: the library it models moves on. The load says so, and the site that
/// rests on it turns plain on the first mismatch — the live procedure answers,
/// where the inlined text would have said otherwise.
#[test]
fn a_pack_text_body_that_diverges_turns_the_site_plain() {
    crate::install_surfaces();
    let (set, _registry) =
        bundled_source("reference-diverged", &reference_pack(DOUBLE_IN_THE_PACK));
    assert!(
        set.notices
            .iter()
            .any(|notice| notice.message.contains("diverges from it silently")),
        "{:#?}",
        set.notices
    );

    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(facts(&set));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2 + 1}}");
    assert_eq!(
        call_caller(&mut vm, "21"),
        "43",
        "the live library's answer, not the pack's"
    );
    assert!(
        plain.get() > 0,
        "the live text differs: refused, recompiled plain"
    );

    // The library catches up with the pack, and the site is admitted again.
    let mut vm = vm_for_overlay();
    let plain = PlainCounting::installed_with(&mut vm, service_for(&set), None);
    vm.set_pack_facts(facts(&set));
    define(&mut vm, "proc vdouble {x} {expr {$x * 2}}");
    assert_eq!(call_caller(&mut vm, "21"), "42");
    assert_eq!(plain.get(), 0);
}

/// A package on disk — a manifest, a pack in `specs/` whose backing points at
/// a file of the package, and that file — and its directory.
fn package_on_disk(name: &str, backing_path: &str, definition: Option<&str>) -> PathBuf {
    package_on_disk_saying(name, backing_path, definition, "")
}

/// The same, with the author asserting that the body may be evaluated.
fn package_on_disk_evaluable(name: &str, backing_path: &str, definition: Option<&str>) -> PathBuf {
    package_on_disk_saying(name, backing_path, definition, " -evaluate")
}

fn package_on_disk_saying(
    name: &str,
    backing_path: &str,
    definition: Option<&str>,
    flag: &str,
) -> PathBuf {
    let dir = scratch(name);
    std::fs::create_dir_all(dir.join("specs")).expect("specs dir");
    std::fs::create_dir_all(dir.join("lib")).expect("lib dir");
    std::fs::write(dir.join("tclpkg.tcl"), "package vendor 1.0\n").expect("manifest");
    let backing = format!("tcl-body {{-package-source {backing_path}{flag}}}");
    std::fs::write(dir.join("specs/vendor.tclspec"), reference_pack(&backing)).expect("pack");
    if let Some(definition) = definition {
        std::fs::write(dir.join("lib/double.tcl"), definition).expect("library file");
    }
    dir
}

fn load_package_pack(dir: &std::path::Path, tier: Option<DependencyTier>) -> PackSet {
    tcl_spectcl::pack::load(&[tcl_spectcl::PackFile {
        tier: tcl_spectcl::Tier::Workspace,
        path: dir.join("specs/vendor.tclspec"),
        origin: tcl_spectcl::discovery::Origin::DotDir,
        dependency_tier: tier,
    }])
}

fn warnings(set: &PackSet) -> Vec<String> {
    set.notices
        .iter()
        .filter(|notice| notice.severity == tcl_spectcl::pack::Severity::Warning)
        .map(|notice| notice.message.clone())
        .collect()
}

/// A body a pack names as a file of its package is read at load, through the
/// store that read the pack, and reaches the compiler as text: the command
/// carries it, the registry holds it, a compile inlines it, and nothing that
/// compiles reads a file. What was read moves the set's key, so a registry
/// built before the file changed is not the one built after.
#[test]
fn a_package_source_body_is_read_at_load_through_the_store() {
    crate::install_surfaces();
    let definition = "proc vdouble {x} {expr {$x * 2}}";
    let dir = package_on_disk("package-source", "lib/double.tcl", Some(definition));
    let set = load_package_pack(&dir, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text.as_deref(), Some(definition));
    assert!(warnings(&set).is_empty(), "{:#?}", set.notices);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
    let spec = registry.get("vdouble").expect("installed");
    assert_eq!(registry.reference_body(spec), Some(definition));

    let module = service_for(&set)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    let caller = fixture_procedure(&module, "::caller");
    assert_eq!(caller.procedure_bindings, vec![double_binding()]);
    assert!(!calls_a_command(caller));

    // The same files give the same key; another library gives another, and
    // the registry for it inlines the library as it now is.
    assert_eq!(
        load_package_pack(&dir, Some(DependencyTier::Root)).key,
        set.key
    );
    std::fs::write(
        dir.join("lib/double.tcl"),
        "proc vdouble {x} {expr {$x * 4}}",
    )
    .expect("library file");
    let moved = load_package_pack(&dir, Some(DependencyTier::Root));
    assert_ne!(moved.key, set.key);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &moved);
    let module = service_for(&moved)
        .compile_for_profile(CALLER, tcl9())
        .expect("compiles");
    assert_eq!(
        fixture_procedure(&module, "::caller").procedure_bindings,
        vec![ProcedureBindingIdentity::new(
            "vdouble",
            "::vdouble",
            "x",
            "expr {$x * 4}"
        )]
    );
}

/// A file that cannot be read, a path that leaves the package and a pack no
/// package ships are each a warning on the command's row; the command keeps
/// its declaration and is not inlined. A load with no store reads nothing and
/// says nothing.
#[test]
fn a_package_source_that_cannot_be_read_is_said_and_not_inlined() {
    crate::install_surfaces();
    let definition = "proc vdouble {x} {expr {$x * 2}}";
    for (name, path, written, expected) in [
        ("package-missing", "lib/double.tcl", None, "No such file"),
        (
            "package-escaping",
            "../double.tcl",
            Some(definition),
            "relative path inside the package",
        ),
        (
            "package-absolute",
            "/etc/hostname",
            Some(definition),
            "relative path inside the package",
        ),
    ] {
        let dir = package_on_disk(name, path, written);
        let set = load_package_pack(&dir, Some(DependencyTier::Root));
        let command = set.packs[0].command("vdouble").expect("declared");
        assert_eq!(command.reference_text, None, "{name}");
        assert!(
            matches!(
                command.spec.runtime_backing,
                tcl_registry::RuntimeBacking::TclBody { .. }
            ),
            "{name}: the declaration stands"
        );
        let said = warnings(&set);
        assert_eq!(said.len(), 1, "{name}: {:#?}", set.notices);
        assert!(said[0].contains(expected), "{name}: {}", said[0]);
        assert!(
            said[0].contains("a call to `vdouble` is not inlined"),
            "{name}: {}",
            said[0]
        );
        let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
        let module = service_for(&set)
            .compile_for_profile(CALLER, tcl9())
            .expect("compiles");
        assert!(
            fixture_procedure(&module, "::caller")
                .procedure_bindings
                .is_empty(),
            "{name}"
        );
    }

    // A pack no package ships has no directory to read from.
    let dir = package_on_disk("package-unshipped", "lib/double.tcl", Some(definition));
    std::fs::remove_file(dir.join("tclpkg.tcl")).expect("remove the manifest");
    let unshipped = load_package_pack(&dir, None);
    let said = warnings(&unshipped);
    assert_eq!(said.len(), 1, "{said:#?}");
    assert!(said[0].contains("no `tclpkg.tcl` above the pack ships it"));

    // A load with no store reads nothing and says nothing.
    let in_memory = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: dir.join("specs/vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        reference_pack("tcl-body {-package-source lib/double.tcl}"),
    )]);
    assert!(warnings(&in_memory).is_empty(), "{:#?}", in_memory.notices);
    let command = in_memory.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text, None);
    // A load that read nothing keeps the key its sources gave it, whether it had
    // a store or not.
    assert_eq!(unshipped.key, in_memory.key);
}

/// The text a package source names is run as the command's declared
/// implementation too, where its author asserted the body may be evaluated: the
/// load reads the file and derives the hook from it as it does from a pack-text
/// body, and a file the load could not read, or a direct dependency's body, which
/// the gate has dropped, supplies none to derive from. The same file without the
/// author's word derives nothing.
#[test]
fn a_package_source_body_is_a_declared_implementation_too() {
    crate::install_surfaces();
    use tcl_registry::value_transfer::SemanticsDeclaration;

    let definition = "proc vdouble {x} {expr {$x * 2}}";
    let dir =
        package_on_disk_evaluable("package-implementation", "lib/double.tcl", Some(definition));
    let own = load_package_pack(&dir, Some(DependencyTier::Root));
    let command = own.packs[0].command("vdouble").expect("declared");
    assert!(
        matches!(command.spec.semantics, SemanticsDeclaration::Declared(_)),
        "{:?}",
        command.spec.semantics
    );
    assert_eq!(command.hooks.len(), 1, "{:?}", command.hooks);

    let silent = package_on_disk(
        "package-implementation-unasserted",
        "lib/double.tcl",
        Some(definition),
    );
    let set = load_package_pack(&silent, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert_eq!(command.reference_text.as_deref(), Some(definition));
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert_eq!(command.hooks, [] as [tcl_spectcl::HookDecl; 0]);
    assert!(warnings(&set).is_empty(), "{:#?}", set.notices);

    let unread = package_on_disk_evaluable("package-implementation-unread", "lib/double.tcl", None);
    let set = load_package_pack(&unread, Some(DependencyTier::Root));
    let command = set.packs[0].command("vdouble").expect("declared");
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert_eq!(command.hooks, [] as [tcl_spectcl::HookDecl; 0]);

    let dependency = load_package_pack(&dir, Some(DependencyTier::Direct));
    let command = dependency.packs[0].command("vdouble").expect("declared");
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Inherited
    ));
    assert_eq!(command.hooks, [] as [tcl_spectcl::HookDecl; 0]);
}

/// The capability matrix's reference-body row at the load: the workspace's own
/// package's body is inlined; a direct dependency's pack loses the backing, with
/// a warning that says why, and nothing of it is inlined; a pack no package
/// ships is not narrowed.
#[test]
fn a_direct_dependencys_reference_body_is_dropped_at_load() {
    crate::install_surfaces();
    let compiled = |tier: Option<DependencyTier>, name: &str| {
        let set = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: scratch(name).join("vendor.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: tier,
            },
            reference_pack(DOUBLE_IN_THE_PACK),
        )]);
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &set);
        let spec = registry.get("vdouble").expect("installed");
        let module = service_for(&set)
            .compile_for_profile(CALLER, tcl9())
            .expect("compiles");
        let inlined = !fixture_procedure(&module, "::caller")
            .procedure_bindings
            .is_empty();
        let refusal = warnings(&set)
            .into_iter()
            .find(|message| message.contains("may not declare a reference body"));
        (spec.runtime_backing, inlined, refusal)
    };

    let (backing, inlined, refusal) = compiled(Some(DependencyTier::Root), "reference-root");
    assert!(matches!(
        backing,
        tcl_registry::RuntimeBacking::TclBody { .. }
    ));
    assert!(inlined);
    assert_eq!(refusal, None);

    let (backing, inlined, refusal) = compiled(Some(DependencyTier::Direct), "reference-direct");
    assert_eq!(backing, tcl_registry::RuntimeBacking::None);
    assert!(!inlined);
    assert!(
        refusal
            .expect("said on the pack file")
            .contains("a direct dependency's pack")
    );

    let (_, inlined, refusal) = compiled(None, "reference-unshipped");
    assert!(inlined);
    assert_eq!(refusal, None);
}
