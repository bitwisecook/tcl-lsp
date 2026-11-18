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

//! The flow-sensitive lattice: baselines, renames, destroys, redefinitions and exception edges.

use super::*;

#[test]
fn catalogue_baseline_retains_rootability_and_exact_descriptor_coordinates() {
    let mut registry = CommandRegistry::build_default();
    assert!(registry.get("my").is_some());
    assert!(registry.get("::my").is_none());
    assert_eq!(default_binding("::my", &registry).kind, BindingKind::Opaque);
    for label in ["expr", "::expr"] {
        assert_eq!(default_binding(label, &registry).kind, BindingKind::Builtin);
    }
    for label in [":expr", "::::expr", "::missing", "::"] {
        assert_eq!(default_binding(label, &registry).kind, BindingKind::Opaque);
    }
    let mut rooted = registry.get_exact("expr").unwrap().clone();
    rooted.name = "::my";
    registry.insert(rooted);
    assert_eq!(
        default_binding("::my", &registry).kind,
        BindingKind::Builtin
    );
}

#[test]
fn consumed_cfg_commands_retain_the_original_native_entry_baseline() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = SourceAnalysisEntry {
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        "set result VALUE",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        Arc::clone(&context),
    );
    let cfg = &unit.top_level.cfg;
    let statement = &cfg.blocks[&cfg.entry].statements[0];
    assert!(matches!(statement, Statement::AssignConst { .. }));
    assert!(statement.tokens().is_none());
    let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &cfg.command_binding_sites,
        statement,
    )
    .unwrap();
    assert!(
        original
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_some()
    );
    let selected = analyse_command_binding(cfg, context.commands(), &[]);
    assert!(selected.is_original_builtin_at(cfg.entry, 0, "set"));
    let mut missing = cfg.clone();
    missing.command_binding_sites.clear();
    assert!(missing.statement_sources.values().any(Option::is_some));
    assert!(
        !analyse_command_binding(&missing, context.commands(), &[]).is_original_builtin_at(
            missing.entry,
            0,
            "set"
        )
    );
    let mut conflicting = cfg.clone();
    let mut other = conflicting.command_binding_sites[0].clone();
    other.source_tokens.as_mut().unwrap().argv_texts[0] = "other".into();
    conflicting.command_binding_sites.push(other);
    assert!(
        !analyse_command_binding(&conflicting, context.commands(), &[]).is_original_builtin_at(
            conflicting.entry,
            0,
            "set"
        )
    );
    let mut foreign = context.commands().project_for_profile(profile);
    let mut setter = foreign.get("set").unwrap().clone();
    setter.lowering_hook = None;
    foreign.insert(setter);
    assert!(
        !analyse_command_binding(cfg, &foreign, &[]).is_original_builtin_at(cfg.entry, 0, "set")
    );
    drop(owner);
}

#[test]
fn unperturbed_builtin_is_builtin_no_rebound() {
    let (cu, reg) = analyse("string toupper a");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert!(cb.is_original_builtin_at(fu.cfg.entry, 0, "string"));
    assert!(cb.rebound_names().is_empty());
    assert!(!cb.has_wildcard());
}

#[test]
fn class_destroy_makes_the_class_command_opaque() {
    // `Animal destroy` deletes the class command: the binding is Class
    // before the destroy and Opaque after, so a later `Animal new`
    // draws W128.  Definer creation and the destructive method are
    // both registry data (definition_body / oo::object's `destroy`).
    let (cu, reg) = analyse(
        "oo::class create Animal {}
Animal new
Animal destroy
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let animal_calls: Vec<_> = fu.cfg.blocks[&entry]
        .statements
        .iter()
        .enumerate()
        .filter_map(|(index, stmt)| {
            matches!(stmt, Statement::Call { command, .. } | Statement::Barrier { command, .. }
                    if command == "Animal")
            .then_some(index)
        })
        .collect();
    assert_eq!(
        cb.binding_at(entry, animal_calls[0], "Animal").kind,
        BindingKind::Class
    );
    assert_eq!(
        cb.binding_at(entry, animal_calls[2], "Animal").kind,
        BindingKind::Opaque,
        "the class command is deleted after `Animal destroy`"
    );
    assert!(cb.rebound_names().contains("::Animal"));
}

#[test]
fn instance_destroy_makes_the_instance_command_opaque() {
    // `Animal create fido` binds the instance command; `fido destroy`
    // deletes it; the class itself stays bound.
    let (cu, reg) = analyse(
        "oo::class create Animal {}
Animal create fido
fido destroy
fido bark
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let invocation_index = |command: &str, first_arg: &str| {
        fu.cfg.blocks[&entry]
            .statements
            .iter()
            .position(|stmt| {
                matches!(stmt,
                        Statement::Call { command: source, args, .. }
                        | Statement::Barrier { command: source, args, .. }
                        if source == command
                            && args.first().is_some_and(|arg| arg == first_arg))
            })
            .unwrap()
    };
    assert_eq!(
        cb.binding_at(entry, invocation_index("fido", "bark"), "fido")
            .kind,
        BindingKind::Opaque
    );
    assert_eq!(
        cb.binding_at(entry, invocation_index("Animal", "new"), "Animal")
            .kind,
        BindingKind::Class
    );
}

#[test]
fn closed_static_descriptor_does_not_mask_a_live_object_receiver() {
    let mut reg = CommandRegistry::build_default();
    reg.insert(tcl_registry::CommandSpec {
        name: "Animal",
        state_transitions: tcl_registry::knowledge::Knowledge::Partial(
            tcl_registry::StateTransitionDescriptor::EMPTY,
        ),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let cu = CompilationUnit::build_for(
        "oo::class create Animal {}\nAnimal destroy\nAnimal new",
        &reg,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert_eq!(
        cb.binding_at(fu.cfg.entry, 2, "Animal").kind,
        BindingKind::Opaque,
        "the live object binding wins over the shadowed static descriptor"
    );
}

#[test]
fn destroy_as_ordinary_argument_is_not_a_deletion() {
    // A proc named `destroy` taking a class name as an ARGUMENT must
    // not delete anything: the head is the proc, not the class.
    let (cu, reg) = analyse(
        "oo::class create Animal {}
proc destroy {x} { puts $x }
destroy Animal
Animal new",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    assert_eq!(
        cb.binding_at(entry, 3, "Animal").kind,
        BindingKind::Class,
        "the class survives an unrelated `destroy` call"
    );
}

#[test]
fn snit_type_creation_has_only_its_attested_dispatcher() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let selected =
        crate::provider_fixtures::entry(registry, &[crate::provider_fixtures::Provider::Snit]);
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "package require snit\nsnit::type Dog {}\nDog create d";
    let offset = u32::try_from(source.find("Dog create").unwrap()).unwrap();
    let bindings =
        SourceCommandBindings::analyse_with_options(source, config, registry, selected.options());
    let call = bindings.invocation_at_source("Dog", offset);
    assert_eq!(
        call.proved_execution_target().unwrap().kind,
        BindingKind::Command
    );
    assert_eq!(
        call.nominal_definition_name_result(registry),
        Some(tcl_registry::definer::DefinitionDispatcher::SnitType)
    );
    let unprovided = SourceCommandBindings::analyse(source, config, registry);
    assert!(
        unprovided
            .invocation_at_source("Dog", offset)
            .nominal_definition_name_result(registry)
            .is_none()
    );
    let retired_source = "package require snit\nsnit::type Dog {}\nDog destroy\nDog create d";
    let retired_offset = u32::try_from(retired_source.find("Dog create").unwrap()).unwrap();
    let retired = SourceCommandBindings::analyse_with_options(
        retired_source,
        config,
        registry,
        selected.options(),
    );
    assert!(
        retired
            .invocation_at_source("Dog", retired_offset)
            .nominal_definition_name_result(registry)
            .is_none()
    );
}

#[test]
fn rename_deletion_makes_old_name_opaque_flow_sensitively() {
    // `string` is its builtin before the rename, opaque after.
    let (cu, reg) = analyse("string toupper a\nrename string {}\nstring toupper b");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    assert!(cb.is_original_builtin_at(entry, 0, "string"));
    assert_eq!(
        cb.binding_at(entry, 2, "string").kind,
        BindingKind::Opaque,
        "string is renamed away before stmt 2"
    );
    assert!(cb.rebound_names().contains("::string"));
}

#[test]
fn rename_redirect_moves_binding_to_new_name() {
    let (cu, reg) = analyse("rename string mystr\nmystr toupper b");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    // After the rename: old `string` is opaque, `mystr` inherits the
    // builtin binding `string` denoted.
    assert_eq!(cb.binding_at(entry, 1, "string").kind, BindingKind::Opaque);
    assert_eq!(cb.binding_at(entry, 1, "mystr").kind, BindingKind::Builtin);
}

#[test]
fn proc_redefinition_binds_name_to_proc() {
    let (cu, reg) = analyse("proc string {x} { return $x }\nstring foo");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    let entry = fu.cfg.entry;
    let b = cb.binding_at(entry, 1, "string");
    assert_eq!(b.kind, BindingKind::Proc);
    assert_eq!(b.target.as_deref(), Some("::string"));
    assert!(!cb.is_original_builtin_at(entry, 1, "string"));
    assert!(cb.rebound_names().contains("::string"));
}

#[test]
fn dynamic_rename_collapses_to_wildcard() {
    let (cu, reg) = analyse("set x foo\nrename $x bar\nstring toupper a");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert!(cb.has_wildcard(), "dynamic rename sets the wildcard");
    // Under the wildcard everything resolves to Unknown (⊤), never a
    // concrete binding — so no spurious W128 can fire.
    let entry = fu.cfg.entry;
    assert_eq!(cb.binding_at(entry, 2, "string").kind, BindingKind::Unknown);
}

#[test]
fn try_handler_joins_command_mutation_from_exception_edge() {
    // A fall-through-capable try body may fail before or after the rename.
    // The analysis-only exception edges therefore contribute both the
    // pre-try Builtin state and the body-exit Opaque state to the handler.
    let (cu, reg) = analyse(
        "proc ::p {} {\n try {\n  rename string {}\n } on error {} {\n  string length abc\n }\n}",
    );
    let fu = cu.function("::p").unwrap();
    let handler = fu
        .cfg
        .blocks
        .iter()
        .find_map(|(&id, block)| block.name.starts_with("try_handler").then_some(id))
        .expect("try handler block");
    assert!(
        fu.cfg
            .exception_edges
            .iter()
            .any(|&(_, target)| target == handler),
        "the test must exercise an analysis-only exception edge"
    );

    let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
    assert_eq!(
        cb.binding_at(handler, 0, "string").kind,
        BindingKind::Unknown,
        "the handler must not infer either the original or renamed binding"
    );
    assert!(
        !cb.is_original_builtin_at(handler, 0, "string"),
        "an optimisation must not trust the builtin at handler entry"
    );
}

#[test]
fn seed_marks_module_procs_as_proc() {
    // The W128 seed: a name seeded as PROC resolves to Proc at entry.
    let (cu, reg) = analyse("nonbuiltin a b");
    let fu = cu.function("::top").unwrap();
    let seed = vec![(
        "::myproc".to_owned(),
        Binding {
            kind: BindingKind::Proc,
            target: Some("::myproc".to_owned()),
        },
    )];
    let cb = analyse_command_binding(&fu.cfg, &reg, &seed);
    assert_eq!(
        cb.binding_at(fu.cfg.entry, 0, "myproc").kind,
        BindingKind::Proc
    );
}

/// `walk_body_calls` recurses once
/// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`
/// body, with no depth cap of its own. Transitively
/// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass,
/// so this is defence-in-depth / consistency with every other
/// full-tree walker in this crate, not a currently-reproducible
/// crash. 1000 levels of source nesting is comfortably past this
/// cap; the assertion is that `scan_module_command_mutations` returns
/// at all, not what it returns. Spawns its own big-stack thread since
/// the lexer/CST/segmenter stages upstream of the lowering cap still
/// walk the full un-truncated source nesting before that cap trims
/// it — same rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_walk_body_calls() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "proc clobber {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("rename string {}\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let reg = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let _ = scan_module_command_mutations(&cu.ir_module, &reg);
        })
        .unwrap()
        .join()
        .unwrap();
}
