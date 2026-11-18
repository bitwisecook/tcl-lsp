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

use super::*;
use crate::command_binding::SourceNamespaceKey;
use crate::var_resolve::VariableCellKey;

#[test]
fn qualified_alias_tail_advice_uses_retained_context_over_catalogue_profile() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This checks authored metadata availability and source-name suppression,
    // without claiming native namespace-link execution or cell identity.
    let catalogue = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let no_tcl = tcl_registry::model::ingress::static_context_for("f5-bigip")
        .with_command_store(std::sync::Arc::clone(catalogue.commands()));
    assert!(std::sync::Arc::ptr_eq(
        catalogue.commands(),
        no_tcl.commands()
    ));
    assert!(
        catalogue
            .context()
            .resolve_spec(catalogue.commands(), "variable")
            .is_some()
    );
    assert!(
        no_tcl
            .context()
            .resolve_spec(no_tcl.commands(), "variable")
            .is_none()
    );
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "variable ns::tail; variable unqualified",
        catalogue.commands(),
        false,
        "tcl8.6",
    );
    let function = &unit.top_level;
    let considered = function.cfg.blocks.keys().copied().collect();
    assert_eq!(
        collect_qualified_variable_alias_tails(function, &considered, catalogue),
        FxHashSet::from_iter(["tail".to_owned()]),
        "positive original qualified alias-tail metadata",
    );
    assert!(collect_qualified_variable_alias_tails(function, &considered, &no_tcl).is_empty());
    let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "proc variable args {}; variable ns::tail",
        catalogue.commands(),
        false,
        "tcl8.6",
    );
    let considered = replaced.top_level.cfg.blocks.keys().copied().collect();
    assert!(
        collect_qualified_variable_alias_tails(&replaced.top_level, &considered, catalogue)
            .is_empty(),
        "an authored replacement cannot borrow namespace-variable metadata",
    );
}

fn occurrence_span(source: &str, spelling: &str) -> tcl_lexer::Span {
    let start = source.rfind(spelling).expect("original occurrence");
    tcl_lexer::Span::new(
        u32::try_from(start).expect("bounded source offset"),
        u32::try_from(start + spelling.len()).expect("bounded source end"),
    )
}

#[test]
fn original_array_scalar_advice_does_not_revive_retired_roots_or_elements() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    for (source, spelling, wrong_kind) in [
        ("proc f {} {set a(k) 1; set copy $a}", "$a", true),
        ("proc f {} {set a(k) 1; unset a; set copy $a}", "$a", false),
        (
            "proc f {} {set a(k) 1; unset a(k); set copy $a(k)}",
            "$a(k)",
            false,
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = cu.procedures.values().next().expect("original procedure");
        assert_eq!(
            original_array_scalar_read_at_span(
                function,
                occurrence_span(source, spelling),
                registry,
            ),
            wrong_kind,
            "{source}"
        );
    }
}

#[test]
fn special_variable_advice_matches_its_original_output_word() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let source = "proc f {} {scan {a b} {%s %s} local ::auto_path}";
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = cu.procedures.values().next().expect("original procedure");
    assert_eq!(
        special_variable_definition_at_span(
            function,
            occurrence_span(source, "::auto_path"),
            registry,
        ),
        Some("auto_path".into())
    );
    assert_eq!(
        special_variable_definition_at_span(function, occurrence_span(source, "local"), registry,),
        None
    );
}

fn footprint_inputs(
    source: &str,
) -> (
    crate::compilation_unit::CompilationUnit,
    crate::analyser::AnalysisResult,
    crate::analyser::ResolvedAnalysisInput,
) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: input.borrowed_context_registry().commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "tcl");
    (unit, analysis, input)
}

fn footprint_semantics<'a>(
    source: &'a str,
    analysis: &'a crate::analyser::AnalysisResult,
    input: &'a crate::analyser::ResolvedAnalysisInput,
) -> super::UndefSuppressionSemantics<'a> {
    super::UndefSuppressionSemantics {
        dialect: Some(
            input
                .borrowed_context_registry()
                .context()
                .authoring_query(),
        ),
        rules: tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
        lexer_config: input.lexer_config(),
        source,
        analysis,
        context: input.borrowed_context_registry(),
        module: None,
    }
}

#[test]
fn materialized_suppression_keeps_original_aliases_and_literal_names() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // The real diagnostic suppression consumer receives possible names,
    // never a Native write, successful evaluation or child argv receipt.
    for (source, expected) in [
        (
            "proc f {} {eval set {café(open} VALUE; puts ${café(open}}",
            Some("café(open"),
        ),
        (
            "interp alias {} emit {} eval set {$literal}; proc f {} {emit VALUE; puts ${$literal}}",
            Some("$literal"),
        ),
        (
            "rename set moved; interp alias {} write {} moved café; proc f {} {eval write VALUE; puts $café}",
            Some("café"),
        ),
        (
            "proc set args {}; proc f {} {eval set hidden VALUE; puts $hidden}",
            None,
        ),
    ] {
        let (unit, analysis, input) = footprint_inputs(source);
        let function = unit.function("::f").unwrap();
        let considered = function.ssa.blocks.keys().copied().collect();
        let names = super::collect_script_concat_writes(
            function,
            &considered,
            footprint_semantics(source, &analysis, &input),
        );
        if let Some(expected) = expected {
            assert!(names.contains(expected), "{source}: {names:?}");
        } else {
            assert!(!names.contains("hidden"), "known replacement: {names:?}");
        }
    }
}

#[test]
fn expression_suppression_uses_each_original_child_lookup_horizon() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    for (source, expected) in [
        (
            "proc f {} {set result [expr {[catch {set {café(open} VALUE} status] || $status}]}",
            Some("café(open"),
        ),
        (
            "interp alias {} assign {} set {$literal}; proc f {} {set result [expr {[assign VALUE]}]}",
            Some("$literal"),
        ),
        (
            "proc f {} {set result [expr {[set x VALUE] + [rename set replaced]}]}",
            Some("x"),
        ),
        (
            "proc f {} {if {[catch {set x VALUE} first]} {} elseif {[set y VALUE]} {}}",
            Some("x"),
        ),
        (
            "proc f {} {if {[catch {set x VALUE} first]} {} elseif {[set y VALUE]} {}}",
            Some("y"),
        ),
        (
            "proc catch args {}; proc f {} {set result [expr {[catch {set hidden VALUE} status]}]}",
            None,
        ),
    ] {
        let (unit, analysis, input) = footprint_inputs(source);
        let function = unit.function("::f").unwrap();
        let considered = function.ssa.blocks.keys().copied().collect();
        let names = super::collect_expr_cmd_sub_writes(
            function,
            &considered,
            footprint_semantics(source, &analysis, &input),
        );
        if let Some(expected) = expected {
            assert!(names.contains_key(expected), "{source}: {names:?}");
        } else {
            assert!(
                !names.contains_key("hidden") && !names.contains_key("status"),
                "known child replacement: {names:?}"
            );
        }
    }
}

#[test]
fn materialized_suppression_refuses_missing_foreign_and_stale_source_input() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    let source = "proc f {} {eval set output VALUE; puts $output}";
    let (unit, analysis, input) = footprint_inputs(source);
    let function = unit.function("::f").unwrap();
    let considered = function.ssa.blocks.keys().copied().collect();
    let semantics = footprint_semantics(source, &analysis, &input);
    assert!(
        super::collect_script_concat_writes(function, &considered, semantics).contains("output")
    );
    let mut missing = function.clone();
    missing.source_metadata_input = None;
    assert!(super::collect_script_concat_writes(&missing, &considered, semantics).is_empty());
    let mut foreign = function.clone();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        input.lexer_config(),
    ));
    assert!(super::collect_script_concat_writes(&foreign, &considered, semantics).is_empty());
    let stale = super::UndefSuppressionSemantics {
        source: "proc f {} {eval set output OTHER; puts $output}",
        ..semantics
    };
    assert!(super::collect_script_concat_writes(function, &considered, stale).is_empty());
}

#[test]
fn original_definition_advice_refuses_missing_and_foreign_function_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These are selected source/API definition places, not measured native stores.
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let source = "proc f {} {scan {a b} {%s %s} local ::auto_path}";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let original = unit.procedures.values().next().unwrap();
    let span = occurrence_span(source, "::auto_path");
    let definitions = |function: &crate::compilation_unit::FunctionUnit| {
        function
            .cfg
            .blocks
            .iter()
            .flat_map(|(&block, body)| {
                (0..body.statements.len()).flat_map(move |index| {
                    original_definition_places(function, block, index, registry)
                })
            })
            .collect::<Vec<_>>()
    };
    assert!(definitions(original).iter().any(|place| {
        crate::var_resolve::root_namespace_variable_simple_name(place) == Some("auto_path")
    }));
    assert_eq!(
        special_variable_definition_at_span(original, span, registry),
        Some("auto_path".to_owned())
    );
    let mut missing = original.clone();
    missing.source_metadata_input = None;
    assert_eq!(definitions(&missing), [] as [crate::place::Place; 0]);
    assert_eq!(
        special_variable_definition_at_span(&missing, span, registry),
        None
    );
    let mut foreign = original.clone();
    let input = original.source_metadata_input().unwrap();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        input.lexer_config(),
    ));
    assert_eq!(definitions(&foreign), [] as [crate::place::Place; 0]);
    assert_eq!(
        special_variable_definition_at_span(&foreign, span, registry),
        None
    );
}

#[test]
fn original_existence_guards_use_retained_availability_and_refuse_missing_foreign() {
    // naming.compiler.retained-existence-metadata
    // docs/design/analysis/name-resolution-proofs/retained-existence-metadata.md
    // Conditional branch advice only; no native query result or current contents proof.
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let info = registry.get("info").unwrap().clone();
    registry.insert(tcl_registry::CommandSpec {
        surface: registry.get("dict").unwrap().surface,
        ..info
    });
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let profile = tcl_registry::model::ingress::static_context_for("tcl")
        .commands()
        .profile()
        .unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "proc f {} {if {[info exists {scalar(open}]} {puts ${scalar(open}}}",
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        context,
    );
    let original = unit.procedures.values().next().unwrap();
    assert!(
        original
            .source_metadata_input()
            .unwrap()
            .has_logical_source_name_context()
    );
    assert_eq!(
        collect_existence_guards(original, &registry, config).len(),
        1
    );
    let input = original.source_metadata_input().unwrap();
    let mut unavailable = original.clone();
    unavailable.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        ),
        input.lexer_config(),
    ));
    assert_eq!(
        collect_existence_guards(&unavailable, &registry, config),
        [] as [(crate::var_resolve::VariableCellKey, crate::cfg::BlockId); 0]
    );
    let mut missing = original.clone();
    missing.source_metadata_input = None;
    assert_eq!(
        collect_existence_guards(&missing, &registry, config),
        [] as [(crate::var_resolve::VariableCellKey, crate::cfg::BlockId); 0]
    );
    let mut foreign = original.clone();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        input.lexer_config(),
    ));
    assert_eq!(
        collect_existence_guards(&foreign, &registry, config),
        [] as [(crate::var_resolve::VariableCellKey, crate::cfg::BlockId); 0]
    );
}

#[test]
fn original_read_killed_match_keeps_alias_cell_and_version() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let source = "proc f {} {set original 1; upvar 0 original linked; unset linked; catch {puts $original}; set unrelated 1; puts $unrelated}";
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = cu.procedures.values().next().expect("original procedure");
    let considered = function.ssa.blocks.keys().copied().collect();
    let module = crate::interprocedural::ModuleProcedures::of_unit(&cu, registry);
    let steps = super::call_steps(function, &considered, Some(&module), registry);
    let killed = build_phi_undef_index(function, &considered, registry, &steps).killed;
    let original = original_read_occurrence_at_span(function, occurrence_span(source, "$original"))
        .expect("original read value");
    let unrelated =
        original_read_occurrence_at_span(function, occurrence_span(source, "$unrelated"))
            .expect("unrelated read value");
    assert!(killed.contains(&original), "{original:?}: {killed:?}");
    assert!(!killed.contains(&unrelated));
    assert!(!killed.contains(&(original.0.clone(), original.1 + 1)));
    let access = function
        .ssa
        .blocks
        .iter()
        .flat_map(|(&block, body)| (0..body.statements.len()).map(move |index| (block, index)))
        .find_map(|(block, index)| {
            let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
            let access = view
                .source_tokens()?
                .variable_accesses
                .iter()
                .find(|access| access.original_spelling == "$original")?;
            Some((
                view.read_reference(&access.source, &access.original_spelling)?,
                view.read_occurrence_reference(&access.source, &access.original_spelling)?,
            ))
        })
        .expect("exact original attempted read");
    assert!(
        access.0.version.is_none(),
        "unset produced no readable value"
    );
    assert_eq!(function.ssa.cell_key(access.1.0), Some(&original.0));
    assert_eq!(access.1.1, original.1);
}

#[test]
fn original_unhandled_missing_read_has_no_unentered_suffix_value_or_contents_receipt() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {set original 1; upvar 0 original linked; unset linked; puts $original; set unrelated 1; puts $unrelated}";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = unit.function("::f").unwrap();
    let span = occurrence_span(source, "$unrelated");
    assert!(original_read_occurrence_at_span(function, span).is_none());
    let mut original_word_retained = false;
    for (&block, body) in &function.ssa.blocks {
        for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for word in tokens.words() {
                if function.abs_span(word.source().span) != span {
                    continue;
                }
                original_word_retained = true;
                assert!(view.read_reference(word.source(), "$unrelated").is_none());
                assert!(
                    view.read_completion_at(word.source(), "$unrelated", registry)
                        .is_none()
                );
                assert!(
                    view.read_contents_presence_at(word.source(), "$unrelated", registry)
                        .is_none()
                );
            }
        }
    }
    assert!(
        original_word_retained,
        "original unentered diagnostic source remains available"
    );
}

#[test]
fn destruction_versions_do_not_kill_a_prior_read_or_supply_a_stored_value() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {set original 1; puts $original; upvar 0 original linked; unset linked; puts $original}";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = unit.function("::f").unwrap();
    let first_start = source.find("$original").unwrap();
    let first = original_read_occurrence_at_span(
        function,
        tcl_lexer::Span::new(
            u32::try_from(first_start).unwrap(),
            u32::try_from(first_start + "$original".len()).unwrap(),
        ),
    )
    .expect("original earlier read");
    let start = source.rfind("$original").unwrap();
    let later = original_read_occurrence_at_span(
        function,
        tcl_lexer::Span::new(
            u32::try_from(start).unwrap(),
            u32::try_from(start + "$original".len()).unwrap(),
        ),
    )
    .expect("original attempted read after destruction");
    let considered = function.ssa.blocks.keys().copied().collect();
    let module = crate::interprocedural::ModuleProcedures::of_unit(&unit, registry);
    let steps = super::call_steps(function, &considered, Some(&module), registry);
    let killed = build_phi_undef_index(function, &considered, registry, &steps).killed;
    assert_eq!(first.0, later.0, "the alias selects the same actual cell");
    assert_ne!(first.1, later.1, "destruction has its own contents version");
    assert!(
        !killed.contains(&first),
        "an earlier readable value was not destroyed then"
    );
    assert!(killed.contains(&later));
    let destruction = function
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| !statement.destruction_defs.is_empty())
        .expect("a distinct destruction transition");
    assert!(
        destruction
            .destruction_defs
            .iter()
            .all(|symbol| destruction.defs.contains_key(symbol))
    );
    for (&block, body) in &function.ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
            for &symbol in &statement.destruction_defs {
                assert!(!view.normal_store_contents_preserved(symbol, registry));
            }
        }
    }
}

#[test]
fn original_read_occurrence_does_not_borrow_an_unknown_alias_cell() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {which} {set a 1; set b 2; upvar 0 $which linked; puts $linked}";
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = unit.function("::f").unwrap();
    assert!(
        original_read_occurrence_at_span(function, occurrence_span(source, "$linked")).is_none(),
        "a written alias label cannot identify its dynamic target cell",
    );
}

#[test]
fn killed_values_supersede_exists_guards_for_scalar_element_and_return() {
    for source in [
        "proc f {x} {if {[info exists x]} {unset x; puts $x}}",
        "proc f {flag x} {if {$flag && [info exists x]} {unset x}; puts $x}",
        "proc f {flag} {set a(k) 1; if {$flag && [info exists a(k)]} {unset a(k)}; puts $a(k)}",
        "proc f {x} {if {[info exists x]} {unset x; return $x}}",
        "proc f {flag x} {if {$flag && [info exists x]} {unset x}; return $x}",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == crate::analyser::types::DiagCode::W210 }),
            "{source}: {:?}",
            result.diagnostics
        );
    }
    for source in [
        "proc f {x} {if {[info exists x]} {puts $x}}",
        "proc f {x} {if {[info exists x]} {return $x}}",
        "proc f {} {set a(k) 1; if {[info exists a(k)]} {puts $a(k)}}",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| { diagnostic.code != crate::analyser::types::DiagCode::W210 }),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

fn namespace_cell(path: tcl_core_types::ByteNamespacePath) -> VariableCellKey {
    VariableCellKey::Namespace {
        identity: SourceNamespaceKey::Native(
            tcl_runtime_api::native_compilation::NativeNamespaceContext {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: 1,
                    interpreter: 1,
                },
                token: 1,
                path,
            },
        ),
        simple: "argv".into(),
    }
}

#[test]
fn startup_facts_require_exact_root_namespace_geometry() {
    let aliases = HashSet::from(["argv".to_owned()]);
    let root = namespace_cell(tcl_core_types::ByteNamespacePath::default());
    assert_eq!(startup_cell_binding(&root, false, &aliases), ("argv", true));
    let named = namespace_cell(tcl_core_types::ByteNamespacePath::default().with_child("::"));
    assert_eq!(
        startup_cell_binding(&named, true, &aliases),
        ("argv", false)
    );
    let local = VariableCellKey::Activation {
        identity: "actual-call".into(),
        simple: "argv".into(),
    };
    assert_eq!(startup_cell_binding(&local, true, &aliases), ("", false));
}

#[test]
fn procedure_namespace_effects_require_original_selected_cells() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let original = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "set ::counter 0\nproc init {} {global counter; set counter 1}\nproc read {} {global counter; return $counter}\ninit; read\nputs $counter",
        registry,
        false,
        "tcl8.6",
    );
    let written = globals_written_by_procs(&original, registry);
    let read = globals_read_by_procs(&original, registry);
    assert!(!written.is_empty(), "selected entered namespace write");
    assert!(!read.is_empty(), "selected entered namespace read");
    assert!(written.intersection(&read).next().is_some());
    // A procedure nothing calls writes nothing: tclsh8.4.20/8.6.18/9.0.4/
    // 9.1.0 `proc init {} {global counter; set counter 1}; puts $counter`
    // -> can't read "counter": no such variable.
    let deferred = crate::analyser::Analyser::new().analyse(
        "proc init {} {global counter; set counter 1}\nproc read {} {global counter; return $counter}\nputs $counter\n",
        "tcl8.6",
    );
    assert!(
        deferred
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == "W210" && d.message.contains("counter")),
        "source declaration metadata cannot establish normal namespace writes: {:?}",
        deferred.diagnostics
    );
    let observed = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "set ::counter 0; proc changed args {error OBSERVED}; trace add variable ::counter write changed\nproc init {} {global counter; set counter 1}\ninit",
        registry,
        false,
        "tcl8.6",
    );
    assert!(
        globals_written_by_procs(&observed, registry).is_empty(),
        "an erroring write observer prevents the successful-definition receipt"
    );
    let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "rename global native_global\nproc global args {}\nproc init {} {global counter; set counter 1}\nproc read {} {set counter LOCAL; return $counter}\ninit; read; puts $counter",
        registry,
        false,
        "tcl8.6",
    );
    assert!(globals_written_by_procs(&replaced, registry).is_empty());
    assert!(globals_read_by_procs(&replaced, registry).is_empty());
}

#[test]
fn namespace_read_advice_does_not_suppress_a_same_named_local() {
    let source = "proc reader {} {return $::counter}\nproc local {} {set counter FIRST; set counter SECOND}\n";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::analyser::types::DiagCode::W211
                && diagnostic.message.contains("counter")
        }),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn safe_initialiser_does_not_exempt_its_substituted_operand() {
    for (source, missing) in [
        ("proc f {} {lappend x item; puts $x}", false),
        ("proc f {} {lappend x $x; puts $x}", true),
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == crate::analyser::types::DiagCode::W210 }),
            missing,
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn root_special_writes_do_not_suppress_same_named_locals() {
    for (source, unused) in [
        ("proc f {} {set auto_path VALUE}", true),
        ("proc f {} {set ::auto_path VALUE}", false),
        ("proc f {} {global auto_path; set auto_path VALUE}", false),
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == crate::analyser::types::DiagCode::W211 }),
            unused,
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn dict_literal_fallback_requires_the_original_reaching_value() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let registry = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    for (source, expected) in [
        (
            "proc f {} {set d {present 1}; dict update d present local {puts $local}}",
            true,
        ),
        (
            "proc f {} {set d {present 1}; unset d; dict update d present local {puts $local}}",
            false,
        ),
        (
            "proc f {} {set d {present 1}; set ::auto_index(unknown_writer) {source lib.tcl}; unknown_writer; dict update d present local {puts $local}}",
            false,
        ),
        (
            "proc f {} {set d {present 1}; unset d; set d {present 2}; dict update d present local {puts $local}}",
            true,
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let function = cu.procedures.values().next().expect("original procedure");
        let considered = function.ssa.blocks.keys().copied().collect();
        let mut suppression = UndefSuppression::default();
        harvest_dict_with_suppression(
            function,
            &considered,
            &mut suppression,
            tcl_syntax::word_rules::WordValueRules::default(),
            &registry,
        );
        assert_eq!(
            suppression.dict_with_known_keys.contains("local"),
            expected,
            "{source}"
        );
    }
}
