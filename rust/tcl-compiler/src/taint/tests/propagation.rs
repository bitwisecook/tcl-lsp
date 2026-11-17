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

//! The propagation pass: entry seeds, incoming external globals, formal seeds, opaque accesses and retained availability.

use super::*;

#[test]
fn opaque_native_accesses_withdraw_receiver_and_clean_taint_proofs() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut unit =
        crate::compilation_unit::CompilationUnit::build_for("set value KNOWN", registry, false);
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    unit.top_level
        .cfg
        .blocks
        .get_mut(&unit.top_level.cfg.entry)
        .unwrap()
        .statements
        .push(native.clone());
    let graph = TaintGraph::new(
        &unit.top_level.cfg,
        &unit.top_level.ssa,
        &unit.top_level.sccp,
    );
    let taints = propagate_taints(
        &graph,
        TaintPropagationInputs {
            registry,
            rendered_props: None,
            interproc: None,
            source: TaintSourceContext::standalone(registry, registry.profile()),
            param_taints: None,
            taint_summaries: None,
            instance_classes: &LocalInstanceClasses::default(),
        },
    );
    let keys: Vec<_> = unit
        .top_level
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .flat_map(|statement| {
            statement
                .defs
                .iter()
                .map(|(&symbol, &version)| (symbol, version))
        })
        .collect();
    assert_ne!(keys, [] as [(crate::ssa::Symbol, u32); 0]);
    assert!(
        keys.iter()
            .all(|key| taints.get(key) == Some(&TaintLattice::tainted()))
    );
    let mut classes = HashMap::from([("widget".into(), HashSet::from(["Entry".into()]))]);
    transfer_instance_statement(
        &mut classes,
        &native,
        registry,
        TaintSourceContext::for_input(registry, None, tcl_lexer::LexerConfig::default()),
        None,
    );
    assert!(classes.is_empty());
}

fn measured_source_taint(
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> TaintLattice {
    let script = &unit.ir_module.top_level;
    let statement = script.statements.last().expect("source statement");
    let tokens = script
        .retained_source_tokens_for_statement(statement)
        .expect("genuine original source vector");
    let value = tokens.argv_texts.last().expect("original value operand");
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: source.dialect,
        source,
        taint_summaries: None,
        instance_classes: None,
        source_position: tokens.argv.first().map(|span| span.start()),
        source_tokens: Some(tokens),
        word_offset: tokens.argv.last().map(|span| span.start()),
        source_word: tokens.word_exprs.last(),
    };
    word_taint(value, &HashMap::new(), &HashMap::new(), ctx)
}

#[test]
fn actual_taint_lattice_keeps_availability_and_refuses_missing_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source-lattice result and owner refusals, with an independently kept
    // Native entry. This test does not execute gets or observe its result.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut catalogue = baseline.commands().project_for_profile(profile);
    let mut descriptor = catalogue.get("string").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    catalogue.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)));
    let registry = current.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set measured [string length [gets stdin]]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    assert!(!measured_source_taint(&unit, registry).is_tainted());
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for context in [Some(older), Some(foreign), None] {
        let changed = context.map(|context| {
            crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                context,
                input.lexer_config(),
            )
        });
        unit.ir_module.source_metadata_input = changed.clone();
        unit.top_level.source_metadata_input = changed;
        assert!(measured_source_taint(&unit, registry).is_tainted());
    }
    unit.top_level.source_metadata_input = Some(input.clone());
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "missing Module owner"
    );
    unit.ir_module.source_metadata_input = Some(input.clone());
    unit.ir_module.lexer_config.strict_quoting = !input.lexer_config().strict_quoting;
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "stale Module grammar"
    );
    unit.ir_module.lexer_config = input.lexer_config();
    unit.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "stale Module profile"
    );
    unit.ir_module.dialect_profile = Some(input.unit_profile());
    assert!(!measured_source_taint(&unit, registry).is_tainted());
}

fn incoming_global_unit(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> (tcl_vm::Vm, crate::compilation_unit::CompilationUnit) {
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(context),
        config,
    );
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut entry = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        config,
        Some(profile),
    );
    entry.native_entry = Some(std::sync::Arc::new(native));
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (owner, unit)
}

#[test]
fn incoming_external_globals_keep_physical_identity_and_independent_provenance() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    for (source, spelling, name, element) in [
        ("expr {$argv + 1}", "$argv", "argv", false),
        (
            "proc writer {} {set ::state 1}; expr {$argv + 1}; writer",
            "$argv",
            "argv",
            false,
        ),
        (
            "expr {$env(PATH) + 1}; proc writer {} {set ::state 1}; writer",
            "$env(PATH)",
            "env",
            true,
        ),
        ("expr {$::argv + 1}", "$::argv", "argv", false),
        (
            "upvar #0 argv argc; expr {$argc + 1}",
            "$argc",
            "argv",
            false,
        ),
    ] {
        let (_owner, unit) = incoming_global_unit(source, &context);
        let unit = unit.with_interprocedural(registry, Some(profile));
        let start = u32::try_from(source.find(spelling).unwrap()).unwrap();
        let span = Span::new(start, start + u32::try_from(spelling.len()).unwrap());
        let read = crate::ssa::original_source_read_at_unit(&unit, span, registry)
            .expect("actual retained top-level read");
        assert!(std::ptr::eq(read.function(), &raw const unit.top_level));
        let cell = read.function().ssa.cell_key(read.symbol()).unwrap();
        let global = cell
            .incoming_root_global(unit.ir_module.source_entry.native_entry.as_deref().unwrap())
            .expect("same-entry incoming root-global cell");
        assert_eq!(global.name().as_bytes(), name.as_bytes());
        assert_eq!(global.is_element(), element);
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(&unit, registry, Some(profile));
        let incoming = solved.top_taints[&(read.symbol(), 0)];
        assert!(incoming.is_tainted(), "{source}: {incoming:?}");
        assert!(
            !incoming.is_conservative_global_seed_only(),
            "independent external source was lost: {source}: {incoming:?}"
        );
        // Unknown represented contents remain a separate warning source.
        assert!(
            find_taint_warnings_for_cu(&unit, registry, Some(profile))
                .iter()
                .any(|warning| warning.code == DiagCode::T100 && warning.span == span)
        );
    }
}

#[test]
fn incoming_global_projection_refuses_foreign_nonroot_and_recreated_storage() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let source = "expr {$argv + 1}";
    let (_owner, unit) = incoming_global_unit(source, &context);
    let read = crate::ssa::original_source_read_at_unit(&unit, Span::new(6, 11), registry)
        .expect("actual retained incoming read");
    let cell = read.function().ssa.cell_key(read.symbol()).unwrap();
    let entry = unit.ir_module.source_entry.native_entry.as_deref().unwrap();
    let (_foreign_owner, foreign) =
        crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert!(cell.incoming_root_global(&foreign).is_none());
    // Refusal-only wrappers around the genuine cell issue no SSA or lifetime proof.
    let recreated = crate::var_resolve::VariableCellKey::Lifetime {
        source: 1,
        cell: Box::new(cell.clone()),
    };
    assert!(recreated.incoming_root_global(entry).is_none());
    let member = crate::var_resolve::VariableCellKey::Element {
        cell: Box::new(recreated),
        index: "key".into(),
    };
    assert!(member.incoming_root_global(entry).is_none());
    assert!(
        crate::var_resolve::VariableCellKey::Authored("argv".to_owned())
            .incoming_root_global(entry)
            .is_none()
    );
    let source = "namespace eval box {variable argv}; upvar #0 box::argv argc; expr {$argc + 1}";
    let (_owner, unit) = incoming_global_unit(source, &context);
    let start = u32::try_from(source.find("$argc").unwrap()).unwrap();
    let read =
        crate::ssa::original_source_read_at_unit(&unit, Span::new(start, start + 5), registry)
            .expect("actual retained non-root read");
    assert!(
        read.function()
            .ssa
            .cell_key(read.symbol())
            .unwrap()
            .incoming_root_global(unit.ir_module.source_entry.native_entry.as_deref().unwrap())
            .is_none()
    );
}

#[test]
fn incoming_external_globals_use_retained_availability_and_entry() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let (_owner, unit) = incoming_global_unit("expr {$argv + 1}", &context);
    let read = crate::ssa::original_source_read_at_unit(&unit, Span::new(6, 11), registry)
        .expect("actual retained incoming read");
    let mut missing_origin = HashMap::new();
    seed_entry_taints(
        &mut missing_origin,
        &unit.top_level.ssa,
        &unit.top_level.cfg,
        None,
        None,
        registry,
        TaintSourceContext::for_function(registry, &unit.top_level),
    );
    assert!(!missing_origin.contains_key(&(read.symbol(), 0)));
    let retained =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    for separate_dialect in [None, Some(tcl_dialect::DialectProfile::irules())] {
        let mut context = retained;
        context.dialect = separate_dialect;
        let mut incoming = HashMap::new();
        seed_entry_taints(
            &mut incoming,
            &unit.top_level.ssa,
            &unit.top_level.cfg,
            None,
            None,
            registry,
            context,
        );
        assert!(incoming[&(read.symbol(), 0)].is_tainted());
    }
    let mut unavailable = retained;
    unavailable.metadata = None;
    assert!(unavailable.incoming_native_global_context().is_none());
    assert!(
        TaintSourceContext::standalone(registry, Some(profile))
            .incoming_native_global_context()
            .is_none()
    );
    let mut detached = unit.clone();
    detached.ir_module.source_entry.native_entry = None;
    assert!(
            TaintSourceContext::for_module_function(
                registry,
                &detached.ir_module,
                &detached.top_level,
            )
            .incoming_native_global_context()
            .is_none()
        );
    detached.ir_module.source_entry = unit.ir_module.source_entry.clone();
    detached.ir_module.source_entry.unknown_entry = true;
    assert!(
            TaintSourceContext::for_module_function(
                registry,
                &detached.ir_module,
                &detached.top_level,
            )
            .incoming_native_global_context()
            .is_none()
        );
}

#[test]
fn incoming_global_metadata_cannot_replace_genuine_formal_or_gets_sources() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let (_owner, unit) = incoming_global_unit("proc p {argv} {expr {$argv + 1}}", &context);
    let (function, symbol) = unit
        .all_body_function_units()
        .find_map(|function| {
            function.ssa.cell_symbols().find_map(|(symbol, cell)| {
                matches!(cell, crate::var_resolve::VariableCellKey::Activation { simple, .. }
                        if simple.as_bytes() == b"argv")
                .then_some((function, symbol))
            })
        })
        .expect("genuine retained formal cell; no exact diagnostic read is claimed");
    let source = TaintSourceContext::for_module_function(registry, &unit.ir_module, function);
    let mut clean = HashMap::new();
    seed_entry_taints(
        &mut clean,
        &function.ssa,
        &function.cfg,
        None,
        None,
        registry,
        source,
    );
    assert!(!clean.contains_key(&(symbol, 0)));
    let mut explicit = HashMap::new();
    seed_entry_taints(
        &mut explicit,
        &function.ssa,
        &function.cfg,
        None,
        Some(&HashMap::from([(
            "argv".to_owned(),
            TaintLattice::tainted(),
        )])),
        registry,
        source,
    );
    assert!(explicit[&(symbol, 0)].is_tainted());
    assert!(!explicit[&(symbol, 0)].is_conservative_global_seed_only());

    for (source, spelling) in [
        ("set argc [gets stdin]; expr {$argc + 1}", "$argc"),
        (
            "set argv SAFE; unset argv; set argv [gets stdin]; expr {$argv + 1}",
            "$argv",
        ),
    ] {
        let (_owner, unit) = incoming_global_unit(source, &context);
        let unit = unit.with_interprocedural(registry, Some(profile));
        let start = u32::try_from(source.find(spelling).unwrap()).unwrap();
        let read = crate::ssa::original_source_read_at_unit(
            &unit,
            Span::new(start, start + u32::try_from(spelling.len()).unwrap()),
            registry,
        )
        .expect("actual retained gets/rebind read");
        let version = read.version().expect("represented later contents");
        assert!(version > 0);
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(&unit, registry, Some(profile));
        let value = solved.top_taints[&(read.symbol(), version)];
        assert!(value.is_tainted());
        assert!(!value.is_conservative_global_seed_only());
    }
}

#[test]
fn version_zero_seed_provenance_does_not_use_display_name() {
    let cfg = crate::cfg::Function::new("::top", "entry");
    let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
    let symbol = ssa.intern_var("::formal");
    let uses = HashMap::from([(symbol, 0)]);
    let global_seed = HashMap::from([((symbol, 0), TaintLattice::conservative_global_seed())]);
    let formal_entry = HashMap::from([((symbol, 0), TaintLattice::tainted())]);
    assert!(!join_uses(&uses, &global_seed).is_tainted());
    assert!(join_uses(&uses, &formal_entry).is_tainted());
    assert!(global_seed[&(symbol, 0)].is_conservative_global_seed_only());
    assert!(
        !global_seed[&(symbol, 0)]
            .join(TaintLattice::tainted())
            .is_conservative_global_seed_only()
    );
}

#[test]
fn source_formal_seed_requires_the_matching_retained_body() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "proc run {formal} {set local $formal}",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let function = unit.procedures.values().next().unwrap();
    let source = TaintSourceContext::for_module_function(registry, &unit.ir_module, function);
    let formal = function.ssa.var_symbol("formal").unwrap();
    let local = function.ssa.var_symbol("local").unwrap();
    assert!(is_actual_formal_entry(
        &function.ssa,
        formal,
        "formal",
        source
    ));
    assert!(!is_actual_formal_entry(
        &function.ssa,
        local,
        "local",
        source
    ));
    let foreign_body =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert!(!is_actual_formal_entry(
        &function.ssa,
        formal,
        "formal",
        foreign_body
    ));
}

#[test]
fn propagate_taints_gets_is_source() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    // Minimal CFG: entry block with `set x [gets stdin]`.
    let stmt = Statement::AssignValue {
        span: Span::new(0, 20),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(stmt.clone());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let ssa_stmt = ssa_stmt(stmt, HashMap::new(), [(x, 1u32)].into_iter().collect());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    assert!(
        taints.get(&(x, 1)).is_some_and(|t| t.is_tainted()),
        "gets stdin result should be tainted"
    );
}

/// Global-write seeding must be scoped to procs actually reachable
/// from the current function. An unrelated helper that writes to
/// `::state` should not taint `::other_global` in a function that
/// never calls it.
#[test]
fn global_write_seeding_is_scoped_to_reachable_callees() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default();
    // `::writer` writes ::state. `::top` never calls ::writer, so
    // reads of other globals here must stay clean.
    let cu = CompilationUnit::build_for(
        "proc ::writer {} { set ::state 1 }\n\
             set local $::safe",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let fu = cu.function("::top").unwrap();
    let local_tainted = fu
        .taints
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "local" && t.is_tainted());
    assert!(
        !local_tainted,
        "`local` must stay clean — `::writer` is unreachable from top-level: {:?}",
        fu.taints,
    );
}

/// A name an opaque `switch`'s arm may overwrite keeps the taint it held
/// when no arm runs: the arm need not run, so `puts $u` still sees data
/// read from the channel.
#[test]
fn a_tainted_name_an_opaque_switch_arm_may_overwrite_stays_tainted() {
    for (release, registry) in release_registries() {
        let source = "proc p {s} {\n set u [gets stdin]\n switch -glob -- $s { q* { set u clean } }\n puts $u\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let found = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            found.iter().any(|w| w.code == DiagCode::T101),
            "{release}: expected T101 for the sink after the switch, got {found:?}"
        );
    }
}
