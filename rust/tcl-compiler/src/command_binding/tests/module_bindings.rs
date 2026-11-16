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

//! The may-state: copy-on-write forks, procedure deltas, embedded heads and namespace resolution.

use super::*;

#[test]
fn binding_state_forks_copy_procedure_inventory_only_for_a_new_body() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let mut fork = original.clone();

    assert!(Arc::ptr_eq(
        &original.procedure_bodies,
        &fork.procedure_bodies
    ));
    assert!(original.same_state(&fork));

    assert!(!fork.extend_procedure_bodies(["::first".to_owned()]));
    assert!(
        Arc::ptr_eq(&original.procedure_bodies, &fork.procedure_bodies),
        "re-observing a retained body must not detach the shared inventory"
    );

    assert!(fork.extend_procedure_bodies(["::recovered".to_owned()]));
    assert!(!Arc::ptr_eq(
        &original.procedure_bodies,
        &fork.procedure_bodies
    ));
    assert!(!original.procedure_bodies.contains("::recovered"));
    assert!(fork.procedure_bodies.contains("::recovered"));
    assert!(!original.same_state(&fork));
}

#[test]
fn binding_state_forks_copy_binding_map_only_for_a_changed_entry() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let mut unchanged = original.clone();

    assert!(Arc::ptr_eq(&original.bindings, &unchanged.bindings));
    assert!(Arc::ptr_eq(
        &original.root_boundary_bindings,
        &unchanged.root_boundary_bindings
    ));
    assert!(!unchanged.join(&original));
    assert!(
        Arc::ptr_eq(&original.bindings, &unchanged.bindings),
        "joining an identical fork must not detach the shared map"
    );

    let (existing_name, existing_bindings) = original
        .bindings
        .iter()
        .next()
        .expect("procedure analysis publishes at least one binding");
    unchanged.replace(existing_name.clone(), existing_bindings.clone());
    assert!(
        Arc::ptr_eq(&original.bindings, &unchanged.bindings),
        "replacing an entry with its current value must not detach the shared map"
    );

    let mut changed = original.clone();
    changed.replace(
        "::recovered".to_owned(),
        BTreeSet::from([MayBinding::Unknown]),
    );
    assert!(!Arc::ptr_eq(&original.bindings, &changed.bindings));
    assert!(!original.bindings.contains_key("::recovered"));
    assert_eq!(
        changed.bindings.get("::recovered"),
        Some(&BTreeSet::from([MayBinding::Unknown]))
    );
    assert!(
        Arc::ptr_eq(
            &original.root_boundary_bindings,
            &changed.root_boundary_bindings
        ),
        "a flow-state mutation must not copy the immutable boundary map"
    );
    assert!(Arc::ptr_eq(
        &original.procedure_bodies,
        &changed.procedure_bodies
    ));
    assert!(!original.same_state(&changed));
}

#[test]
fn exact_procedure_delta_preserves_history_and_root_boundary() {
    let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let history = bindings
        .bindings
        .get("::first")
        .expect("historical procedure slot");
    let procedure_target = history
        .iter()
        .find_map(|binding| match binding {
            MayBinding::Target(target) if target.kind == BindingKind::Proc => Some(target),
            MayBinding::Target(_)
            | MayBinding::Imported(_)
            | MayBinding::Missing
            | MayBinding::Unknown => None,
        })
        .expect("actual installed procedure");
    assert_eq!(procedure_target.command, "::first");
    assert!(!procedure_target.registry_backed);
    assert_eq!(
        procedure_target.prepended,
        [] as [crate::registry_invocation::EffectiveInvocationWord; 0]
    );
    let allocation = procedure_target
        .implementation_allocation
        .as_ref()
        .expect("actual definition allocation");
    assert_eq!(allocation.site.offset, 0);
    assert_eq!(allocation.command, "::first");
    assert_eq!(
        procedure_target
            .token
            .as_ref()
            .and_then(|identity| identity.allocation.as_ref()),
        Some(allocation)
    );
    let procedure_target = MayBinding::Target(procedure_target.clone());

    assert_eq!(
        bindings.bindings.get("::first"),
        Some(&BTreeSet::from([
            MayBinding::Missing,
            procedure_target.clone()
        ])),
        "the historical view includes the state before and after definition"
    );
    assert_eq!(
        bindings.root_boundary_bindings.get("::first"),
        Some(&BTreeSet::from([procedure_target])),
        "only the post-definition binding is replayable at another root"
    );
}

#[test]
fn exact_procedure_delta_retains_non_binding_opacity() {
    let mut reg = CommandRegistry::build_default();
    let mut external_proc = reg.get("proc").expect("core proc spec").clone();
    external_proc.traits |= tcl_registry::Traits::LOADS_EXTERNAL_UNIT;
    reg.insert(external_proc);

    // The leading command ensures the observed state already exists when
    // the exact definition runs. A one-key fast path here would suppress
    // the ordinary observation and lose the external-unit opacity that
    // accompanies the otherwise-exact Define(Procedure).
    let cu = CompilationUnit::build_for("set marker 1\nproc created {} {return ok}", &reg, false);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(bindings.opaque_binding_mutation);
    assert!(bindings.has_opaque_domain());
    assert!(
        bindings.mutation_projection(&reg).has_dynamic_mutation(),
        "the historical mutation projection must retain external-unit opacity"
    );
}

#[test]
fn exact_procedure_delta_retains_immediate_body_binding_mutation() {
    let mut reg = CommandRegistry::build_default();
    let mut immediate_proc = reg.get("proc").expect("core proc spec").clone();
    immediate_proc
        .traits
        .remove(tcl_registry::Traits::DEFERS_BODY);
    immediate_proc.frame_effect = Some(tcl_registry::FrameEffectSpec {
        level_word: tcl_registry::FrameLevelWord::None,
        layout: tcl_registry::FrameArgLayout::ScriptInCurrentFrame,
    });
    reg.insert(immediate_proc);

    // This authored Proc-lowering operation both defines `created` and
    // immediately evaluates its body. The nested definition changes a
    // second binding without necessarily growing any non-binding axis:
    // both procedure bodies can already be present in the module inventory.
    let cu = CompilationUnit::build_for(
        "set marker 1\nproc created {} {proc set {} {return hijacked}}",
        &reg,
        false,
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(
        !bindings.mutation_projection(&reg).trusts("set"),
        "the historical view must retain the immediately evaluated body's builtin redefinition"
    );
}

#[test]
fn narrow_proc_binding_projection_excludes_unavailable_body_opacity() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let (cu, reg) = analyse(
        "set ::auto_index(missing_command) {source lib.tcl}\nmissing_command argument\nproc retained {} {return ok}",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(
        bindings.mutation_projection(&reg).has_dynamic_mutation(),
        "an unresolved command may load code that changes builtin trust"
    );
    assert!(
        bindings
            .proc_binding_trust_projection()
            .trusts_proc_binding("::retained"),
        "unavailable code does not itself prove that a retained proc was rebound"
    );
}

#[test]
fn narrow_proc_binding_projection_tracks_dynamic_rebinding_subjects() {
    let (cu, reg) = analyse("set name retained\nrename $name saved\nproc retained {} {return ok}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let trust = bindings.proc_binding_trust_projection();

    assert!(trust.has_dynamic_binding_transition());
    assert!(!trust.trusts_proc_binding("::retained"));
}

#[test]
fn narrow_proc_binding_projection_retains_static_namespace_candidates() {
    let (cu, reg) = analyse("namespace eval ::n {rename retained saved}");
    let trust = ModuleCommandBindings::analyse(&cu.ir_module, &reg).proc_binding_trust_projection();

    assert!(!trust.has_dynamic_binding_transition());
    assert!(!trust.trusts_proc_binding("::n::retained"));
    assert!(
        !trust.trusts_proc_binding("::retained"),
        "the legacy source scan also records a global fallback candidate"
    );
}

#[test]
fn expanded_embedded_rename_widens_module_bindings() {
    for (source, opaque) in [
        ("set ignored [rename {*}{llength saved_llength}]", false),
        ("set argv [read stdin]; set ignored [rename {*}$argv]", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_llength", "::")
                    .iter()
                    .any(|target| target.command == "::llength"),
                "{source}"
            );
        }
    }
}

#[test]
fn expanded_embedded_command_head_widens_module_bindings() {
    for (source, opaque) in [
        ("set ignored [{*}{rename string saved_string}]", false),
        ("set argv [read stdin]; set ignored [{*}$argv]", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_string", "::")
                    .iter()
                    .any(|target| target.command == "::string"),
                "{source}"
            );
        }
    }
}

#[test]
fn embedded_non_body_ensemble_leaf_preserves_command_trust() {
    let (cu, reg) = analyse("proc p {} {set x [namespace qualifiers ::a::b]}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a resolved non-body namespace leaf must not inherit the root's dynamic-body opacity"
    );
    let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(mutations.trusts("namespace"));
    assert!(mutations.trusts("self"));
}

#[test]
fn namespace_code_captures_direct_and_embedded_bodies_without_replaying_them() {
    for source in [
        "namespace code {rename set saved_set}",
        "set body {rename set saved_set}\nset callback [namespace code $body]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "namespace code stores rather than executes its body: {source}"
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "a captured namespace code body must not mutate bindings now: {source}"
        );
    }
}

#[test]
fn evaluated_body_interpreter_realm_preserves_only_proven_parent_bindings() {
    for source in [
        "interp create slave\ninterp eval slave {set x 99}",
        "interp create slave\ninterp eval slave {puts child}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a harmless readable body in a named child leaves the parent command table unchanged: {source}"
        );
    }

    for source in [
        "interp eval {} {rename set saved_set}",
        "set child slave\ninterp eval $child {set x 99}",
        "interp create slave\ninterp eval slave {rename set saved_set}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a current, unknown, or command-mutating child body must fail closed: {source}"
        );
    }
}

#[test]
fn namespace_resolution_projection_preserves_written_runs_and_constructed_colons() {
    for (pattern, expected) in [
        ("::source:::tail", ":::::tail"),
        ("::source::a:b", ":::::a:b"),
        ("::source::", ":::::"),
    ] {
        let transition = NamespaceTransition::Forget {
            namespace: tcl_registry::NamespaceTransitionTarget::Current,
            patterns: vec![TransitionSubject::Literal(pattern.to_owned())],
        };
        let mut rebound = std::collections::HashSet::new();
        let mut changed = false;
        let mut opaque = std::collections::HashSet::new();
        assert!(collect_namespace_resolution(
            &transition,
            Some(":::"),
            &mut rebound,
            &mut changed,
            &mut opaque,
        ));
        assert!(changed);
        assert!(opaque.is_empty());
        assert_eq!(
            rebound,
            std::collections::HashSet::from([expected.to_owned()])
        );
    }
}

#[test]
fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {
    for source in [
        "namespace eval ::n {}",
        "namespace eval ::n {namespace export *}",
        "set name v\nglobal $name",
        "set name v\nvariable $name",
        "set ns ::n\nnamespace upvar $ns v local",
        "set pattern *\nnamespace export $pattern",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a namespace or variable-cell transition that preserves lookup must stay closed: {source}"
        );
    }

    for source in [
        "namespace delete ::n",
        "namespace import -force ::m::*",
        "namespace forget ::m::*",
        "namespace path {::m}",
        "namespace unknown handler",
        "set value ::m\nnamespace path $value",
        "set value handler\nnamespace unknown $value",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a resolution-changing namespace transition must fail closed: {source}"
        );
    }
}

#[test]
fn namespace_resolution_impact_survives_embedding_and_alias_prefixes() {
    for source in [
        "set result [namespace delete ::n]",
        "set result [namespace import -force ::m::*]",
        "set result [namespace forget ::m::*]",
        "set result [namespace path {::m}]",
        "set result [namespace unknown handler]",
        "interp alias {} mutate {} namespace delete\nmutate ::n",
        "interp alias {} mutate {} namespace import -force\nmutate ::m::*",
        "interp alias {} mutate {} namespace forget\nmutate ::m::*",
        "interp alias {} mutate {} namespace path\nmutate {::m}",
        "interp alias {} mutate {} namespace unknown\nmutate handler",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "resolved nested dispatch must retain namespace lookup impact: {source}"
        );
        assert!(
            scan_module_command_mutations(&cu.ir_module, &reg).changes_command_resolution(),
            "resolved nested dispatch must reach the shared mutation projection: {source}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
            "namespace lookup provenance must not become an unrelated dynamic proc rebinding: {source}"
        );
    }

    for source in [
        "set result [namespace export *]",
        "interp alias {} ensure_n {} namespace eval ::n {}\nensure_n",
        "interp alias {} export_any {} namespace export\nexport_any *",
    ] {
        let (cu, reg) = analyse(source);
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "Ensure and Export preserve existing command resolution: {source}"
        );
    }
}

#[test]
fn external_unit_execution_widens_direct_and_embedded_command_bindings() {
    for source in [
        "source external.tcl",
        "set result [source external.tcl]",
        "load extension.so",
        "set result [load extension.so]",
        "auto_load widget",
        "set result [auto_load widget]",
        "auto_import pkg::*",
        "set result [auto_import pkg::*]",
        "package require Example",
        "set result [package require Example]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "unavailable external code may mutate any command binding: {source}"
        );
    }
}

/// Run from a script, the stock `unknown` only tries `auto_load` and then
/// raises "invalid command name" (`library/init.tcl`, Tcl 8.4.20 through
/// 9.1.0). With no autoload index the analysis can see, an unresolved
/// dispatch defines nothing and the command table stays known. Writing an
/// index input, or a handler whose body is not the stock one, keeps the
/// table open.
#[test]
fn an_unresolved_dispatch_widens_only_through_a_visible_autoload_index() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let opaque = |source: &str| {
            let reg = CommandRegistry::build_default()
                .project_for_profile(tcl_dialect::DialectProfile::find(dialect).unwrap());
            let cu = CompilationUnit::build_for(source, &reg, false);
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain()
        };
        for source in [
            "unknown missing_command",
            "missing_command argument",
            "set result [missing_command argument]",
            "catch {missing_command argument}\nproc local_command {} {return ok}\nlocal_command",
            "rename unknown {}\nmissing_command argument",
            "interp alias {} unknown {} puts\nmissing_command argument",
        ] {
            assert!(!opaque(source), "{dialect}: {source}");
        }
        for source in [
            "set auto_index(missing_command) {source lib/missing.tcl}\nmissing_command",
            "lappend auto_path lib\nmissing_command",
            "proc unknown {command args} {rename $command saved_command}\nmissing_command",
        ] {
            assert!(opaque(source), "{dialect}: {source}");
        }
    }
}

#[test]
fn a_missing_root_binding_dispatches_through_the_live_unknown_handler() {
    let (cu, reg) = analyse(
        "rename unknown old_unknown\n\
             interp alias {} unknown {} rename expr\n\
             proc maybe {} {}\n\
             rename maybe {}\n\
             maybe",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.rebound_names().any(|name| name == "::expr"),
        "calling the missing command must execute the replacement unknown prefix and move expr"
    );
    assert!(
        bindings
            .targets("maybe", "::")
            .iter()
            .any(|target| target.command == "::expr"),
        "the unknown prefix must move expr onto the missing spelling"
    );

    let (cu, reg) = analyse("rename unknown {}\nmissing_command");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(bindings.targets("missing_command", "::").is_empty());
    assert!(!bindings.target_may_be_unknown("missing_command", "::"));
}

#[test]
fn indeterminate_ensemble_dispatch_widens_direct_and_embedded_bodies() {
    for source in [
        "set op eval\nnamespace $op ::n {rename set saved_set}",
        "set op eval\nset result [namespace $op ::n {rename set saved_set}]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a dynamic subcommand can select a body-bearing leaf: {source}"
        );
    }
}
