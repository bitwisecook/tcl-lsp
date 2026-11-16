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
use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present, fixture_proc};
use tcl_registry::CommandRegistry;

use crate::interprocedural::InterproceduralAnalysis;

#[test]
fn return_literal_replacement_keeps_original_head_and_result_word_owner() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    for (source, expected) in [
        ("::return $result", "::return {Hello World}"),
        ("return [::expr {2 + 3}]", "return {Hello World}"),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let statement = module.top_level.statements.last().unwrap();
        assert!(matches!(
            statement,
            Statement::Return {
                value_word: Some(_),
                ..
            }
        ));
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        ctx.ir_module = Some(&module);
        let (span, replacement) =
            original_return_value_replacement(&ctx, statement, tokens, "Hello World").unwrap();
        assert_eq!(
            span,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap())
        );
        assert_eq!(replacement, expected);
        ctx.source = "return foreign";
        assert!(
            original_return_value_replacement(&ctx, statement, tokens, "Hello World").is_none()
        );
    }
}

#[test]
fn return_source_word_constructor_keeps_head_and_rejects_multiple_words() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    for (source, expected) in [
        ("::return $result", "::return [::expr {$x}]"),
        ("return [::expr {2 + 3}]", "return [::expr {$x}]"),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let statement = module.top_level.statements.last().unwrap();
        assert!(matches!(
            statement,
            Statement::Return {
                value_word: Some(_),
                ..
            }
        ));
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let mut ctx = PassContext::new(source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        ctx.ir_module = Some(&module);
        let (span, replacement) =
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]").unwrap();
        assert_eq!(
            span,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap())
        );
        assert_eq!(replacement, expected);
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]; unwanted")
                .is_none()
        );
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "{*}[::expr {$x}]").is_none()
        );
        ctx.source = "return foreign";
        assert!(
            original_return_word_replacement(&ctx, statement, tokens, "[::expr {$x}]").is_none()
        );
    }
}

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn proc_fold_context(cu: &CompilationUnit, name: &str) -> String {
    let summary = cu.interproc.as_ref().and_then(|ia| fixture_proc(ia, name));
    let summary = summary.map(|s| {
        (
            s.pure,
            s.can_fold_static_calls,
            s.has_barrier,
            s.has_unknown_calls,
            s.writes_global,
            &s.constant_return,
        )
    });
    let body = cu.function(name).map(|fu| {
        let statements: Vec<_> = fu
            .cfg
            .blocks
            .iter()
            .map(|(id, block)| {
                let kinds: Vec<_> = block
                    .statements
                    .iter()
                    .map(std::mem::discriminant)
                    .collect();
                (
                    id,
                    kinds,
                    block.terminator.as_ref().map(std::mem::discriminant),
                )
            })
            .collect();
        (statements, &fu.sccp.values, &fu.sccp.executable_blocks)
    });
    format!("summary(pure,fold,barrier,unknown,global,return)={summary:?}, body={body:?}")
}

fn run_pass(source: &str) -> Vec<Optimisation> {
    let reg = registry();
    let cu = CompilationUnit::build_for(source, &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    // Mirror `optimise_unit_raw`'s production wiring — `run_load_forwarding`
    // (O102) needs `ctx.registry` for its intervening-call purity scan, and
    // the trace/alias safety gates need `ctx.ir_module`. Without these two
    // fields set, a hand-built `PassContext` silently loses O102 coverage
    // (and O127's pre-existing trace-purity gate), which is exactly the bug
    // this test helper would otherwise mask.
    ctx.registry = Some(&reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn propagation_literal_words_retain_selected_grammar_and_exact_unicode_value() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        for value in [
            "",
            "42",
            "has space",
            "$literal [data]",
            "brace}tail",
            "λ",
            "line\nnext",
            "trailing\\",
        ] {
            let word = render_propagation_word(value, config).unwrap();
            assert!(tcl_syntax::word_rules::lexical_unicode_source_word_matches(
                &word, value, config
            ));
        }
    }
}

#[test]
fn resolve_proc_qname_walks_namespace_chain() {
    // Two procs in `::ns`; a bare `inner` call from `::ns` resolves to
    // `::ns::inner`, not the (absent) root `::inner`.
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::ns {\n proc inner {} { return 1 }\n proc outer {} { inner }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(
        fixture_id_if_present(&ia.procedures, "::ns::inner").is_some(),
        "expected ::ns::inner in IA, got {:?}",
        ia.procedures.keys().collect::<Vec<_>>(),
    );
    assert_eq!(
        resolve_proc_qname("inner", "::ns", &ia).as_deref(),
        Some("::ns::inner"),
        "bare call should resolve against the enclosing namespace",
    );
    // An absolute spelling is taken as-is.
    assert_eq!(
        resolve_proc_qname("::ns::inner", "::other", &ia).as_deref(),
        Some("::ns::inner"),
    );
    // A bare call with no matching namespace candidate fails to resolve.
    assert_eq!(resolve_proc_qname("missing", "::ns", &ia), None);
    // From the root namespace a bare `inner` does *not* reach `::ns::inner`.
    assert_eq!(resolve_proc_qname("inner", "::", &ia), None);
}

#[test]
fn resolve_proc_qname_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — current
    // namespace, then global — absent an explicit `namespace path`.
    // `::a::b::c::caller`'s body calling bare `foo` must NOT resolve to
    // a `::a::foo` defined in a *grandparent* namespace: real tclsh
    // raises "invalid command name" there, it does not silently walk up
    // to find `::a::foo`.
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::a {\n proc foo {} { return 1 }\n namespace eval b {\n  namespace eval c {\n   proc caller {} { foo }\n  }\n }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(
        fixture_id_if_present(&ia.procedures, "::a::foo").is_some(),
        "expected ::a::foo in IA, got {:?}",
        ia.procedures.keys().collect::<Vec<_>>(),
    );
    assert_eq!(
        resolve_proc_qname("foo", "::a::b::c", &ia),
        None,
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: the *direct* enclosing namespace still resolves.
    let module2 = crate::lowering::lower_to_ir(
        "namespace eval ::a::b::c {\n proc foo {} { return 1 }\n proc caller {} { foo }\n}",
        &registry(),
    );
    let ia2 = crate::interprocedural::build_interprocedural_analysis(
        &module2,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert_eq!(
        resolve_proc_qname("foo", "::a::b::c", &ia2).as_deref(),
        Some("::a::b::c::foo"),
    );
}

#[test]
fn resolve_proc_qname_relative_dotted_word_prefers_current_namespace() {
    // A relative *dotted* word (`ns2::inner`, containing `::` but not
    // starting with it) must resolve against the current namespace
    // first, not be rooted straight at global — the same rule
    // `bareword_resolution_candidates` documents (confirmed against
    // tclsh 9.0.4). Two procs of this shape exist here, one nested
    // under `::ns` and one at the root; a bare `ns2::inner` call from
    // inside `::ns` must reach the nearer one.
    let module = crate::lowering::lower_to_ir(
        "proc ns2::inner {} { return 1 }\n\
             namespace eval ::ns {\n proc ns2::inner {} { return 2 }\n proc caller {} { ns2::inner }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(fixture_id_if_present(&ia.procedures, "::ns2::inner").is_some());
    assert!(fixture_id_if_present(&ia.procedures, "::ns::ns2::inner").is_some());
    assert_eq!(
        resolve_proc_qname("ns2::inner", "::ns", &ia).as_deref(),
        Some("::ns::ns2::inner"),
        "a relative dotted call must prefer the current-namespace proc \
             over the root one",
    );
    // Falls back to the root proc when there is no current-namespace
    // candidate.
    assert_eq!(
        resolve_proc_qname("ns2::inner", "::other", &ia).as_deref(),
        Some("::ns2::inner"),
    );
}

/// Run the whole optimiser (raw, unfiltered) so the registry is
/// threaded — needed for the O127 store-to-load-forwarding pass,
/// which bails without a registry.
fn o127(source: &str) -> Vec<Optimisation> {
    crate::optimiser::optimise_raw_for_profile(
        source,
        &registry(),
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code == DiagCode::O127)
    .collect()
}

#[test]
fn o127_forwards_single_use_computed_assignment() {
    // `set x [llength $y]` is computed and used once → inline at the
    // use site + delete the store, both O127 in one group.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
    let opts = o127(src);
    assert_eq!(opts.len(), 2, "{opts:?}");
    // Both edits share a group id (all-or-nothing).
    assert_eq!(opts[0].group, opts[1].group);
    assert!(opts[0].group.is_some());
    // One edit inlines `[set x [llength $y]]`; the other deletes.
    assert!(
        opts.iter()
            .any(|o| o.replacement.contains("[set x [llength $y]]"))
    );
    assert!(opts.iter().any(|o| o.replacement.is_empty()));
}

#[test]
fn o127_operand_selection_follows_the_actual_alias_cell() {
    use crate::ssa::SsaSourceView;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for names in [("value", "pointer"), ("pointer", "value")] {
        let source = format!(
            "proc p {{input}} {{set {} [llength $input]; set other [llength $input]; upvar 0 {} {}; puts ${{{}}}}}",
            names.0, names.0, names.1, names.1
        );
        let unit = CompilationUnit::build_for(&source, registry, false);
        let function = unit.function("::p").unwrap();
        let mut checked = false;
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let Some(tokens) = statement.statement.tokens() else {
                    continue;
                };
                if tokens.argv_texts.first().map(String::as_str) != Some("puts") {
                    continue;
                }
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let operand = &tokens.words()[1];
                let read = view.read_word(operand).unwrap();
                let version = read.version.unwrap();
                let (span, prior_effect) = locate_use_var(
                    tokens,
                    view,
                    read.symbol,
                    version,
                    registry,
                    unit.ir_module.lexer_config,
                )
                .unwrap();
                assert!(!prior_effect);
                let span = tcl_lexer::word_span_at(&source, span);
                assert_eq!(
                    &source[span.start() as usize..span.end() as usize],
                    format!("${{{}}}", names.1)
                );
                let other = view.reaching_binding("other", registry).unwrap().symbol;
                assert_ne!(other, read.symbol);
                assert!(
                    locate_use_var(
                        tokens,
                        view,
                        other,
                        version,
                        registry,
                        unit.ir_module.lexer_config
                    )
                    .is_none()
                );
                checked = true;
            }
        }
        assert!(checked);
        assert!(
            o127(&source).is_empty(),
            "an aliased receiver cannot move its store"
        );
    }
}

#[test]
fn o127_observed_alias_receiver_keeps_its_original_store() {
    let source = "proc observe args {}; proc p {input} {set value [llength $input]; upvar 0 value pointer; trace add variable pointer write observe; puts $pointer}";
    assert_eq!(o127(source), [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn o127_skips_intervening_side_effect() {
    // The intervening `puts` is impure → forwarding past it is
    // unsafe.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts hi\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_intervening_upframe() {
    // An intervening `uplevel` lowers to a `Statement::UpFrame`,
    // whose opaque body can reach into / out of this frame and
    // mutate any name the forwarded `[llength $y]` reads.  It must
    // suppress the forward exactly like a barrier — otherwise the
    // re-evaluated inline could compute a different value.
    let src = "proc p {y} {\n    set x [llength $y]\n    uplevel 1 {incr ::n}\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_command_substitution_before_endpoint_use() {
    // Tcl substitutes words left-to-right. Moving the `llength` past the
    // preceding substitution would make it observe the emptied `y`.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts [set y {}] $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_skips_multiple_uses() {
    // Two uses of `$x` → not single-use, no forwarding.
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

#[test]
fn o127_forwards_past_extra_terminator_use() {
    // `$x` has one operand use (`baz $x`) plus a terminator use
    // (`return $x`). Def-use records the terminator use, but it must not
    // block the forward — the forward still fires into the operand use
    // while preserving the assignment for the `return`. The inlined
    // `[set x …]` keeps `x` defined for the trailing `return`.
    let src = "proc baz {value} {return $value}\nproc p {y} {\n    set x [llength $y]\n    baz $x\n    return $x\n}\n";
    let opts = o127(src);
    assert_eq!(opts.len(), 2, "{opts:?}");
    assert!(
        opts.iter()
            .any(|o| o.replacement.contains("[set x [llength $y]]")),
        "{opts:?}"
    );
}

#[test]
fn o127_declines_a_later_unproved_call_after_an_opaque_endpoint() {
    let source = "proc p {y} {set x [llength $y]; baz $x; return $x}";
    assert_eq!(o127(source), [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn o127_skips_literal_assignment() {
    // A literal `set x 5` is the O102 path, not O127.
    let src = "proc p {} {\n    set x 5\n    puts $x\n}\n";
    assert_eq!(o127(src).len(), 0, "{:?}", o127(src));
}

// internal helpers

/// The `${…}` closer comes from the shared owner, so the constant this
/// rewrite inlines belongs to the name the lexer spanned.
///
/// The rewritten text is written back into the document as an O100 fix, so
/// resolving the wrong name substitutes some other variable's value into
/// the user's string.
#[test]
fn substitute_dollar_refs_follows_the_release_close_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let braced = |braced_var| tcl_lexer::LexerConfig {
        braced_var,
        ..tcl_lexer::LexerConfig::default()
    };
    let mut c = std::collections::HashMap::new();
    c.insert("a{b}c".to_owned(), "NESTED".to_owned());
    c.insert("a{b".to_owned(), "FIRST".to_owned());

    // 9.x names `a{b}c`; the whole reference is consumed.
    assert_eq!(
        substitute_dollar_refs("v=${a{b}c}", &c, braced(Tcl9Nesting)).as_deref(),
        Some("v=NESTED")
    );
    // 8.x names `a{b`, and `c}` stays as literal word text.
    assert_eq!(
        substitute_dollar_refs("v=${a{b}c}", &c, braced(FirstClose)).as_deref(),
        Some("v=FIRSTc}")
    );
    // An unterminated reference declines the whole rewrite rather than
    // inventing a name that runs to end-of-input.
    assert!(substitute_dollar_refs("v=${a{b", &c, braced(Tcl9Nesting)).is_none());
}

#[test]
fn interpolation_uses_retained_close_policy_over_catalogue_profile() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Lexical rewrite software control, not variable/frame or Native evidence.
    let source = "\"v=${a{b}c}\"";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut context =
        PassContext::with_dialect(source, InterproceduralAnalysis::default(), Some(profile));
    let original_config = context.lexer_config();
    context.source_lexer_config = Some(tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });
    let registry_context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = registry_context.commands();
    let module_for = |config| {
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&registry_context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input);
        lowerer.lower(source);
        lowerer.finish_module(source)
    };
    let selected_module = module_for(context.lexer_config());
    context.registry = Some(registry);
    context.ir_module = Some(&selected_module);
    let constants = std::collections::HashMap::from([
        ("a{b}c".into(), "NESTED".into()),
        ("a{b".into(), "FIRST".into()),
    ]);
    let config = context.lexer_config();
    visit_string_interpolation(
        &mut context,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        source,
        &constants,
        config,
    );
    assert_eq!(context.optimisations.len(), 1);
    assert_eq!(context.optimisations[0].replacement, "\"v=NESTED\"");
    context.source_lexer_config = None;
    let original_module = module_for(original_config);
    context.ir_module = Some(&original_module);
    context.optimisations.clear();
    let config = context.lexer_config();
    visit_string_interpolation(
        &mut context,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        source,
        &constants,
        config,
    );
    assert_eq!(context.optimisations.len(), 1);
    assert_eq!(context.optimisations[0].replacement, "\"v=FIRSTc}\"");
}

#[test]
fn interpolation_element_indices_keep_literal_braced_names_and_dynamic_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let constants = std::collections::HashMap::from([
        ("a($idx)".into(), "ELEMENT".into()),
        ("a($idx".into(), "SCALAR".into()),
    ]);
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    assert_eq!(
        substitute_dollar_refs("v=$a($idx)", &constants, config).as_deref(),
        Some("v=$a($idx)")
    );
    assert_eq!(
        substitute_dollar_refs("v=${a($idx)}", &constants, config).as_deref(),
        Some("v=ELEMENT")
    );
    assert_eq!(
        substitute_dollar_refs("v=${a($idx}", &constants, config).as_deref(),
        Some("v=SCALAR")
    );
}

#[test]
fn retained_substitution_inventory_keeps_module_input_config_and_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional original child geometry, without handler or erasure permission.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        expand_syntax: false,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        "expr {[dict size [dict create key value]]}",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    let module = &unit.ir_module;
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let calls = retained_substitution_calls(tokens, module, context.commands()).unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .all(|call| call.tokens.as_ref().is_some_and(|tokens| tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.invocation_site().is_some())))
    );
    for change in 0..5 {
        let mut changed = module.clone();
        match change {
            0 => changed.retained_source_bindings = None,
            1 => changed.source_entry.unknown_entry = !changed.source_entry.unknown_entry,
            2 => changed.top_level_namespace = "::other".into(),
            3 => {
                changed.native_namespace =
                    Some(tcl_core_types::ByteNamespacePath::from_segments(["other"]));
            }
            4 => changed.top_level_kind = crate::ir::TopLevelKind::ProcedureBody,
            _ => unreachable!(),
        }
        assert!(
            crate::registry_invocation::InvocationMetadataContext::for_module(
                context.commands(),
                &changed
            )
            .is_some()
        );
        assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    }
    let mut changed = module.clone();
    changed.lexer_config.expand_syntax = true;
    assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(context.commands())),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), context.commands()));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for supplied in [Some(older), Some(foreign), None] {
        changed = module.clone();
        changed.source_metadata_input = supplied.map(|availability| {
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, availability, config)
        });
        assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    }
}

#[test]
fn simple_var_ref_parses_bare_and_braced() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    assert_eq!(
        whole_scalar_variable_reference_name("$foo", config),
        Some("foo")
    );
    assert_eq!(
        whole_scalar_variable_reference_name("${bar}", config),
        Some("bar")
    );
    assert!(whole_scalar_variable_reference_name("$foo(idx)", config).is_none());
    assert!(whole_scalar_variable_reference_name("plain", config).is_none());
}

#[test]
fn safe_bare_word_accepts_int_and_safe_words() {
    assert!(is_value_safe_bare_word("42"));
    assert!(is_value_safe_bare_word("-7"));
    assert!(is_value_safe_bare_word("foo_bar"));
    assert!(!is_value_safe_bare_word(""));
    assert!(!is_value_safe_bare_word("has space"));
    assert!(!is_value_safe_bare_word("$dollar"));
}

// end-to-end tests

#[test]
fn constant_int_propagates_into_call_arg() {
    let opts = run_pass("set x 42\nputs $x");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "42"),
        "expected O100 propagating 42, got {opts:?}",
    );
}

#[test]
fn unsafe_string_constant_is_not_propagated() {
    // Tcl metacharacters in the value → must not inline as
    // a bare word.
    let opts = run_pass("set x \"$other\"\nputs $x");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "unsafe string should not be propagated, got {opts:?}",
    );
}

#[test]
fn o100_multi_word_constant_renders_via_quoter() {
    // A constant containing whitespace or
    // metacharacters is re-rendered as a single self-contained word
    // via the canonical quoter instead of bailing.
    // `return` value position → `return {Hello World}`.
    let ret = run_pass("proc ::f {} { set msg {Hello World}\nreturn $msg }");
    assert!(
        ret.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "return {Hello World}"),
        "expected O100 `return {{Hello World}}`, got {ret:?}",
    );
    // Command-argument position → `puts {Hello World}`.
    let arg = run_pass("proc ::f {} { set msg {Hello World}\nputs $msg }");
    assert!(
        arg.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "{Hello World}"),
        "expected O100 `{{Hello World}}` arg, got {arg:?}",
    );
    // A `$`/`[`-bearing literal constant is brace-quoted (the value
    // is literal — braces suppress the would-be substitution).
    let meta = run_pass("proc ::f {} { set m {a$b}\nputs $m }");
    assert!(
        meta.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "{a$b}"),
        "expected O100 `{{a$b}}` arg, got {meta:?}",
    );
}

#[test]
fn non_const_lattice_skipped() {
    // Two different writes to x → ConstSet or Overdefined,
    // not a single Const.
    let opts = run_pass("set x 1\nif {$cond} { set x 2 }\nputs $x");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "non-const lattice should be skipped, got {opts:?}",
    );
}

#[test]
fn braced_var_reference_also_propagated() {
    let opts = run_pass("set x 7\nputs ${x}");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "7"),
        "expected O100 for braced var ref, got {opts:?}",
    );
}

#[test]
fn return_literal_and_compound_names_do_not_borrow_scalar_constants() {
    for source in [
        "proc f {} {set x 42; return {$x}}",
        "proc f {} {set a 42; set a:b 99; return $a:b}",
        "proc f {} {set arr 42; return ${arr(k)}}",
    ] {
        let optimisations = run_pass(source);
        assert!(
            optimisations.iter().all(|optimisation| {
                optimisation.code != DiagCode::O100
                    || !optimisation.replacement.starts_with("return ")
            }),
            "{source}: {optimisations:?}"
        );
    }
}

#[test]
fn return_terminator_folds_constant_variable() {
    let opts = run_pass("proc ::f {} { set x 42; return $x }");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
        "expected O100 folding return $x to return 42, got {opts:?}",
    );
}

#[test]
fn static_proc_call_folds_to_constant_return() {
    // ::answer returns 42 unconditionally and is pure — a
    // call to ::answer can be folded.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu =
        CompilationUnit::build_for("proc ::answer {} { return 42 }\n::answer", &registry, false)
            .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().any(|o| o.code == DiagCode::O103),
        "expected O103 static-proc fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn return_expr_does_not_fold_stale_pre_loop_constant() {
    // `x` is overwritten by the loop, so its exit version at the return is
    // a non-Const phi (Overdefined). `::f` is pure but has no
    // argument-independent constant return, so `[::f]` reaches the
    // argument-sensitive fold's reading of a returned `expr`
    // (`interprocedural::exit_value`). That
    // fold must NOT leak the pre-loop `set x 0`: tclsh returns 3 (x ends
    // at 2), so folding `[::f]` to 1 would be a miscompile.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc ::f {} { set x 0; foreach v {1 2} { set x $v }; return [expr {$x + 1}] }\nputs [::f]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O103),
        "loop-overwritten var must not fold from a stale pre-loop constant, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn return_expr_after_unresolved_handler_does_not_fold_seeded_argument() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc p {x} {missing_command; return [expr {$x + 1}]}\nputs [p 5]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O103),
        "a handler can change the seeded argument: {:?}",
        ctx.optimisations
    );
}

#[test]
fn return_expr_folds_argument_sensitive_constant() {
    // Control: the argument-sensitive Path 3 still folds when the return
    // expr genuinely depends only on the constant call arguments — the
    // documented `[::add 2 4]` → 6 case (params bound at version 0).
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc ::add {a b} { return [expr {$a + $b}] }\nputs [::add 2 4]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "6"),
        "expected O103 folding [::add 2 4] to 6, got {:?}; {}",
        ctx.optimisations,
        proc_fold_context(&cu, "::add"),
    );
}

#[test]
fn string_interpolation_const_is_inlined() {
    // `set count 42` → Const(42) in SCCP → interpolation into
    // `"count is $count"` substitutes to `"count is 42"`.
    let opts = run_pass("set count 42\nputs \"count is $count\"");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
        "expected O100 inlining count into interpolation, got {opts:?}",
    );
}

#[test]
fn braced_expr_cmd_sub_arg_folds_under_constants() {
    // `puts [expr {$a + $b}]` with a, b proven constant folds the braced
    // expr in the call-argument position to its value.
    let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
        "expected O101 folding [expr {{$a + $b}}] to 7, got {opts:?}",
    );
}

#[test]
fn shadowed_mathfunc_cmd_sub_arg_not_folded() {
    // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin
    // everywhere in the module — the cmd-sub-argument O101 fold path
    // (try_o101_expr_arg_fold) must not fold `abs(-5)` using builtin
    // semantics.
    let opts = run_pass("proc ::tcl::mathfunc::abs {x} { return 999 }\nputs [expr {abs(-5)}]");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a shadowed math function in a cmd-sub argument: {opts:?}",
    );
}

#[test]
fn shadowed_mathfunc_return_value_not_folded() {
    // FP guard: `return [expr {…}]` folding must also respect the
    // math-function shadow gate.
    use tcl_registry::CommandRegistry;
    let reg = CommandRegistry::build_default();
    let source =
        "proc ::tcl::mathfunc::abs {x} { return 999 }\nproc f {} { return [expr {abs(-5)}] }";
    let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&reg);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a shadowed math function in a return value: {:?}",
        ctx.optimisations,
    );
}

#[test]
fn quoted_expr_cmd_sub_arg_not_folded_under_constants() {
    // `puts [expr "$a + $b"]` is the quoted form — it uses textual
    // substitution, so we conservatively never fold it.
    let opts = run_pass("set a 3\nset b 4\nputs [expr \"$a + $b\"]");
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O101 && o.replacement == "7")),
        "quoted expr must not fold under constants, got {opts:?}",
    );
}

/// Like [`run_pass`] but populates `ctx.command_mutations` from the
/// whole module, the way the real pipeline does — needed to exercise
/// the `expr`-redefinition trust gate.
fn run_pass_with_mutations(source: &str) -> Vec<Optimisation> {
    let reg = registry();
    let cu = CompilationUnit::build_for(source, &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    // Same production wiring `run_pass` above documents: the top-level
    // body's `scan_module_global_names` set is registry-resolved, so a
    // context without a registry is deliberately not propagated at all.
    ctx.registry = Some(&reg);
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn renamed_expr_cmd_sub_arg_not_folded() {
    // FP guard: once `expr` is renamed anywhere in the module,
    // `puts [expr {$a + $b}]` no longer calls the builtin evaluator.
    let opts =
        run_pass_with_mutations("rename expr real_expr\nset a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a cmd-sub through a renamed expr: {opts:?}",
    );
}

#[test]
fn renamed_expr_assign_expr_substitution_not_folded() {
    // FP guard: `set x [expr {$a + $b}]` (the AssignExpr form,
    // try_substitute_assign_expr) must also respect the trust gate.
    let opts = run_pass_with_mutations("rename expr real_expr\nset a 3\nset x [expr {$a + 1}]");
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O100 || o.code == DiagCode::O101)),
        "must not propagate/fold set x [expr {{…}}] through a renamed expr: {opts:?}",
    );
}

#[test]
fn ordinary_expr_cmd_sub_arg_still_folds_under_mutation_scan() {
    // TN/control: an unrelated module-wide mutation scan (no rename of
    // `expr` itself) must not block the fold.
    let opts = run_pass_with_mutations("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
        "expected O101 fold under an unrelated mutation scan, got {opts:?}",
    );
}

#[test]
fn renamed_expr_return_value_not_folded() {
    // FP guard: `return [expr {…}]` folding (try_fold_return_terminator)
    // must also respect the trust gate.
    use tcl_registry::CommandRegistry;
    let reg = CommandRegistry::build_default();
    let source = "rename expr real_expr\nproc f {} { return [expr {1 + 2}] }";
    let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a renamed expr in a return value: {:?}",
        ctx.optimisations,
    );
}

#[test]
fn string_interpolation_unknown_var_skipped() {
    // `$name` is not in the constants map → must not fire.
    let opts = run_pass("puts \"hello $name\"");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O100),
        "unknown var should not be inlined, got {opts:?}",
    );
}

#[test]
fn substitute_dollar_refs_handles_braced_and_missing_vars() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "42".into());
    assert_eq!(
        substitute_dollar_refs("a${x}b", &c, tcl_lexer::LexerConfig::default()).as_deref(),
        Some("a42b"),
    );
    // Missing var → None (cannot partially substitute).
    assert!(substitute_dollar_refs("$unknown", &c, tcl_lexer::LexerConfig::default()).is_none());
}

/// Inside a command substitution a braced word substitutes nothing, so
/// its `$x` stays; a `$x` elsewhere, including one after a mid-word
/// brace, is inlined. tclsh 8.4.20 through 9.1b0 print `b:{$x}`,
/// `b:{a $x b}`, `m:a{5}` and `w:{5} {$x} 5` for these (#2250).
#[test]
fn substitute_dollar_refs_leaves_a_braced_word_in_a_cmd_sub_as_written() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "5".into());
    let sub = |text: &str| substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default());
    for (text, want) in [
        ("b:[list {$x}]", "b:[list {$x}]"),
        ("b:[list {a $x b}]", "b:[list {a $x b}]"),
        ("b:[list [list {$x}]]", "b:[list [list {$x}]]"),
        ("s:[set y 1;list {$x}]", "s:[set y 1;list {$x}]"),
        ("c:[list {a]b} $x]", "c:[list {a]b} 5]"),
        ("b:[list {a\\}b $x}]", "b:[list {a\\}b $x}]"),
        ("m:[list a{$x}]", "m:[list a{5}]"),
        ("w:{$x} [list {$x}] $x", "w:{5} [list {$x}] 5"),
        // A `{` inside a quoted word of the substituted script is literal
        // text, and Tcl substitutes the `$x` after it (found in review).
        ("q:[list \"foo {$x}\"]", "q:[list \"foo {5}\"]"),
        ("q:[list \"a]b\" $x]", "q:[list \"a]b\" 5]"),
    ] {
        assert_eq!(sub(text).as_deref(), Some(want), "{text}");
    }
    // A braced word that never closes rewrites nothing.
    assert!(sub("b:[list {$x]").is_none_or(|r| r == "b:[list {$x]"));
}

/// Unchanged text keeps its bytes: copying a byte at a time as `char`
/// turned `é` into `Ã©` (#2251).
#[test]
fn substitute_dollar_refs_keeps_non_ascii_text() {
    let mut c = std::collections::HashMap::new();
    c.insert("x".into(), "5".into());
    for (text, want) in [
        ("café $x", "café 5"),
        ("é\\t$x€", "é\\t5€"),
        ("v:[string length é$x]", "v:[string length é5]"),
    ] {
        assert_eq!(
            substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default()).as_deref(),
            Some(want),
            "{text}"
        );
    }
}

#[test]
fn substitute_dollar_refs_ends_the_empty_braced_name_at_its_brace() {
    let mut c = std::collections::HashMap::new();
    c.insert(String::new(), "5".into());
    for (text, want) in [
        ("a=${}b", "a=5b"),
        ("a=${}", "a=5"),
        ("${}${}", "55"),
        ("v:[list ${}]c", "v:[list 5]c"),
    ] {
        assert_eq!(
            substitute_dollar_refs(text, &c, tcl_lexer::LexerConfig::default()).as_deref(),
            Some(want),
            "{text}"
        );
    }
}

#[test]
fn substitute_dollar_refs_guards_multiword_in_cmd_sub() {
    let mut c = std::collections::HashMap::new();
    c.insert("lst".into(), "a b c".into());
    c.insert("n".into(), "5".into());
    // A multi-word value in plain string text inlines verbatim (it stays
    // part of the one string word).
    assert_eq!(
        substitute_dollar_refs("x=$lst", &c, tcl_lexer::LexerConfig::default()).as_deref(),
        Some("x=a b c"),
    );
    // The SAME value inside a `[…]` command substitution would split one
    // argument into several — bail, keeping `$lst`.
    assert!(
        substitute_dollar_refs("r: [lsearch $lst x]", &c, tcl_lexer::LexerConfig::default())
            .is_none()
    );
    // A single-word value inside a cmd-sub is still safe to inline.
    assert_eq!(
        substitute_dollar_refs("r: [expr $n + 1]", &c, tcl_lexer::LexerConfig::default())
            .as_deref(),
        Some("r: [expr 5 + 1]"),
    );
}

#[test]
fn whole_word_cmd_sub_not_wrapped_as_string_interpolation() {
    // Regression: `puts [expr {$a + $b}]` is a free-standing command
    // substitution, not a `"…"` string. The interpolation path must not
    // wrap it in quotes / mis-span it (which produced the corrupt
    // `puts "[expr {3 + 4}]"]`). No O100 string-interpolation rewrite may
    // fire on a whole-word `[…]`.
    let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .all(|o| o.replacement != "\"[expr {3 + 4}]\"" && !o.replacement.contains("\"]")),
        "whole-word cmd-sub must not be string-interpolated, got {opts:?}",
    );
}

#[test]
fn list_constant_not_inlined_into_string_cmd_sub() {
    // Regression: `[lsearch -exact $tokens uic]` must NOT become
    // `[lsearch -exact tran 1n 100n uic uic]` — that splits the list into
    // separate args and errors at runtime (`bad option "tran"`).
    let opts = run_pass("set tokens {tran 1n 100n uic}\nputs \"r: [lsearch -exact $tokens uic]\"");
    assert!(
        opts.iter()
            .all(|o| !o.replacement.contains("tran 1n 100n uic uic")),
        "list literal must not be word-split into the cmd-sub, got {opts:?}",
    );
}

#[test]
fn load_forwarding_fires_o102_for_single_reaching_def() {
    // `set n 7; puts $n` — single reaching def is literal 7
    // → emit O102 on the puts use site.
    let opts = run_pass("set n 7\nputs $n");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O102),
        "expected O102 load-forwarding, got {opts:?}",
    );
}

/// A definition whose value SCCP proved still forwards.
///
/// `set a 1; incr a` leaves `a` provably `2`, and that is what a use of it
/// should see. O102 only ever recognised a *syntactic* literal as a
/// reaching definition, so the chain stopped at the `incr` and the whole
/// snippet optimised to nothing. The name-keyed `sccp_constants_for`
/// projection cannot rescue it either: `a` holds `1` at one version and
/// `2` at another, so that map drops the variable entirely.
#[test]
fn a_computed_definition_forwards_the_constant_sccp_proved() {
    for (source, want) in [
        ("set a 1\nincr a\nputs \"$a\"\n", "2"),
        ("set a 1\nincr a\nputs $a\n", "2"),
        ("set a 1\nincr a 5\nputs $a\n", "6"),
    ] {
        let opts = run_pass(source);
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && !o.hint_only && o.replacement == want),
            "expected an applicable O100 inlining {want:?} for {source:?}, got {opts:?}",
        );
    }
}

/// A hint-only forward records no replacement.
///
/// Its span is the whole consuming statement, so the literal is not a
/// valid replacement for it — recording one anyway is what let a hint on
/// `incr a` read as a one-click rewrite to a bare `1`.
#[test]
fn a_hint_only_forward_carries_no_replacement() {
    let opts = run_pass("set n 7\nputs \"n=$n\"\n");
    let hints: Vec<&Optimisation> = opts.iter().filter(|o| o.hint_only).collect();
    assert!(!hints.is_empty(), "expected a hint-only O102, got {opts:?}");
    assert!(
        hints.iter().all(|o| o.replacement.is_empty()),
        "a hint with no sub-span target must carry no payload, got {hints:?}",
    );
}

/// A variable-*name* argument is not an operand, so no propagation code
/// may target it.
///
/// `set a 1; incr a` has one reaching literal for `a`, and `incr a`'s only
/// mention of `a` names the cell it mutates. Forwarding the literal there
/// produced `O102 … → 1` over the whole of `incr a`: a rewrite that
/// destroys the increment and leaves `1` in command position, where tclsh
/// answers `invalid command name "1"`. It was unreachable behind the
/// `hint_only` guard, but it was a wrong payload sitting one guard away
/// from being applied, and wrong advice about the statement either way.
#[test]
fn no_o1xx_targets_a_variable_name_argument() {
    // One row per registry role that names a variable rather than passing
    // a value: `VarWrite` read-modify-write, `VarRead`, and the
    // destroying form. Each is the *only* mention of the variable in its
    // statement, so any O1xx on that line is targeting the name.
    for source in [
        "set a 1\nincr a\n",
        "set a 1\nappend a x\n",
        "set a 1\nlappend a x\n",
        "set a 1\ndict incr a k\n",
        "set a 1\ninfo exists a\n",
        "set a 1\nunset a\n",
        "set a 1\nbinary scan xy c a\n",
        "set a 1\nregexp {(x)} xy -> a\n",
    ] {
        let opts = run_pass(source);
        let forwarded: Vec<&Optimisation> = opts
            .iter()
            .filter(|o| o.code.as_str().starts_with("O1") && o.replacement == "1")
            .collect();
        assert_eq!(
            forwarded.len(),
            0,
            "no O1xx may forward the literal into a variable-name position \
                 of {source:?}, got {forwarded:?}",
        );
    }
}

/// The guard is about the *position*, not the command: the same commands
/// still forward into a genuine operand word beside the name.
#[test]
fn a_value_operand_beside_a_name_argument_still_forwards() {
    let opts = run_pass("set n 7\nset a {}\nlappend a $n\n");
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O102),
        "the `$n` operand is still an operand, got {opts:?}",
    );
}

// A computed variable name (`set $name 2`) between the
// "sole" reaching def and the use can rewrite `x` under a spelling the
// def-use chains cannot see: `f x` prints 2 in tclsh, so forwarding the
// literal 1 (via O102's chain scan or O100's SCCP projection) is a
// miscompile. The whole function abstains.
#[test]
fn dynamic_name_write_blocks_load_forwarding() {
    let opts = run_pass("proc ::f {name} { set x 1; set $name 2; puts $x }");
    assert!(
        opts.iter()
            .all(|o| !matches!(o.code, DiagCode::O100 | DiagCode::O102)),
        "must not forward the pre-dynamic-write literal 1, got {opts:?}",
    );
}

/// The native-stack depth cap is not evidence that no
/// dynamic write exists below it. Keep the actual O100/O102 consumer in
/// this regression: the source mutation is hidden past the cap, yet it
/// must still prevent forwarding the stale `x = 1` into the return.
#[test]
fn depth_exhaustion_blocks_dynamic_name_load_forwarding() {
    let mut nested = "[set $name 2]".to_owned();
    for _ in 0..=crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.0 {
        nested = format!("[list {nested}]");
    }
    let source = format!("proc ::f {{name}} {{ set x 1; set sink {nested}; return $x }}");
    let reg = registry();
    let cu = CompilationUnit::build_for(&source, &reg, false);
    assert_eq!(
        cu.function("::f")
            .expect("procedure analysed")
            .dynamic_names,
        crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT,
        "the depth cap must fail closed before the optimiser runs"
    );
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.registry = Some(&reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    let opts = ctx.optimisations;
    assert!(
        opts.iter().all(|opt| {
            !matches!(opt.code, DiagCode::O100 | DiagCode::O102) || opt.replacement != "return 1"
        }),
        "a hidden dynamic write must block the stale return fold: {opts:?}"
    );
}

// Regression: a top-level `global` name reassigned by a proc call must
// never be folded as if its "sole reaching def" were stable — on tclsh
// 8.6/9.0, `set tcl_precision 4; proc helper {} {global tcl_precision;
// set tcl_precision 17}; helper; puts $tcl_precision` prints `17`, not
// the `4` a sole-reaching-def fold would give.  See
// `crate::var_observability::scan_module_global_names`.
#[test]
fn o102_does_not_forward_top_level_global_reassigned_by_callee() {
    let src = "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\nputs $g";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "4")),
        "must not fold the stale pre-call literal 4, got {opts:?}",
    );
}

// TN control: an ordinary proc that itself `global`-declares the name it
// reassigns is *already* protected via SCCP/O100's per-function escaping
// set — O102's independent def-use-chain path must apply the identical
// guard rather than bypassing it (this fired incorrectly before O102
// consulted `var_observability` at all).
#[test]
fn o102_does_not_forward_proc_local_global_reassigned_by_callee() {
    let src = "proc p {} {\n    global g\n    set g 4\n    helper\n    puts $g\n}\nproc helper {} { global g\nset g 17 }\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "4")),
        "must not fold the stale pre-call literal 4, got {opts:?}",
    );
}

// Precision control: a top-level name that *no* procedure ever
// `global`-declares must still fold — the whole-module scan must not
// over-widen to every top-level variable.
//
// Note this now folds via O100, not O102: the intervening call to
// `other` is a command O102 cannot prove pure (it has no interprocedural
// purity or upvar-reach information — a callee can dynamically alias
// any name in the caller's frame via `upvar` with no textual
// `global`/`variable` declaration anywhere), so O102's own independent
// same-block scan conservatively withholds. SCCP (O100) already proves
// `safe_const` a genuine whole-function constant via its own escaping
// set (which correctly found no alias/trace anywhere for it), so the
// fold still happens end to end.
#[test]
fn o102_still_forwards_top_level_global_no_proc_touches() {
    let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\nputs $safe_const";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .any(|o| matches!(o.code, DiagCode::O100 | DiagCode::O102) && o.replacement == "42"),
        "expected an untouched top-level constant to still fold, got {opts:?}",
    );
}

// TN control: a plain proc-local variable (never `global`/`variable`
// declared) is genuinely private to that call frame — O102 must still
// forward it.
#[test]
fn o102_still_forwards_genuinely_local_proc_variable() {
    let src = "proc p {} {\n    set n 7\n    puts $n\n}\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O102 && o.replacement == "7"),
        "expected O102 to still fold a genuinely private proc-local var, got {opts:?}",
    );
}

// Regression: a literal-body `interp eval {}` (targeting the *current*
// interpreter — Tcl's documented meaning of the empty-string path) can
// reassign a variable in the calling scope exactly like an opaque call,
// even though it carries no `global`/`variable`/`upvar`/`trace`
// declaration for it. Confirmed against tclsh 8.6/9.0: `set n 5; interp
// eval {} {set n 99}; puts $n` prints `99`, not the stale `5`.
#[test]
fn o102_does_not_forward_past_interp_eval_barrier() {
    let src = "set n 5\ninterp eval {} { set n 99 }\nputs $n\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
        "must not forward the stale literal 5 past `interp eval {{}}`, got {opts:?}",
    );
}

// Regression: a literal-body `uplevel #0 {…}` evaluates in the absolute
// global frame — at top level that coincides with the calling scope, so
// it can reassign a variable there too, with no alias declaration of its
// own. Confirmed against tclsh 8.6/9.0: `set n 5; uplevel #0 {set n
// 99}; puts $n` prints `99`.
#[test]
fn o102_does_not_forward_past_uplevel_hash0_upframe() {
    let src = "set n 5\nuplevel #0 { set n 99 }\nputs $n\n";
    let opts = run_pass(src);
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
        "must not forward the stale literal 5 past `uplevel #0`, got {opts:?}",
    );
}

#[test]
fn o102_operand_span_fires_applicable_on_call_argv() {
    // `set n 7; puts $n` — the O102 emitted on the puts use
    // must target the `$n` argv span (not the whole Call) and
    // be applicable (not hint-only). The argv span lands on
    // the last 2 chars of the source: `$n`.
    let source = "set n 7\nputs $n";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(
        !o102s.is_empty(),
        "expected at least one O102, got {opts:?}"
    );
    let found = o102s.iter().any(|o| {
        let start = o.span.start() as usize;
        let end = o.span.end() as usize;
        !o.hint_only && o.replacement == "7" && end <= source.len() && &source[start..end] == "$n"
    });
    assert!(
        found,
        "expected applicable O102 with argv span covering `$n`, got {o102s:?}"
    );
}

#[test]
fn o102_operand_span_matches_braced_var_word() {
    // ${n} forms must also be recognised at the argv-text
    // level and produce an applicable rewrite.
    let source = "set n 7\nputs ${n}";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(
        o102s.iter().any(|o| !o.hint_only
            && o.replacement == "7"
            && &source[o.span.start() as usize..o.span.end() as usize] == "${n}"),
        "expected applicable O102 over ${{n}}, got {o102s:?}",
    );
}

#[test]
fn o102_falls_back_to_hint_only_when_var_is_inside_interpolation() {
    // `$n` appears only inside a double-quoted composite word
    // — no argv text matches the bare `$n` form, so the
    // rewrite stays hint-only (the O100 string-interpolation
    // path handles the actual inlining).
    let source = "set n 7\nputs \"n=$n\"";
    let opts = run_pass(source);
    let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
    assert!(!o102s.is_empty(), "expected O102 hint, got {opts:?}");
    assert!(
        o102s.iter().all(|o| o.hint_only),
        "expected all O102s to be hint-only for interpolated use, got {o102s:?}",
    );
}

#[test]
fn o103_cmd_subst_fires_applicable_rewrite() {
    // `[::answer]` inside an argv → applicable O103 with the
    // argv span covering `[::answer]` and the constant return
    // as replacement.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::answer {} { return 42 }\nputs [::answer]";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s.iter().any(|o| !o.hint_only
            && o.replacement == "42"
            && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
        "expected applicable O103 spanning `[::answer]`, got {o103s:?}",
    );
}

#[test]
fn o103_folds_multi_word_string_return_list_quoted() {
    // A pure proc returning a multi-word
    // string folds in a cmd-sub position, list-quoted as one word
    // (a bare `a b c` is not a safe single word).
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::greet {} { return \"a b c\" }\nputs [::greet]";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s
            .iter()
            .any(|o| !o.hint_only && o.replacement == "{a b c}"),
        "expected applicable O103 folding [::greet] to {{a b c}}, got {o103s:?}",
    );
}

#[test]
fn o103_cmd_subst_fires_on_set_value_target() {
    // The value-position cmd-sub folds must also
    // fire on a `set TARGET [cmd-sub]` target. `set` lowers to
    // `AssignValue`, which `walk_statement` did not visit for cmd-sub
    // folds, so `set y [::answer]` missed the O103 fold that the same
    // `[::answer]` gets in a command-argument position (`puts
    // [::answer]`). With the AssignValue arm wired, the set value
    // folds identically.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::answer {} { return 42 }\nproc ::f {} { set y [::answer]\nputs $y }";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s.iter().any(|o| !o.hint_only
            && o.replacement == "42"
            && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
        "expected applicable O103 spanning the `[::answer]` set value, got {o103s:?}",
    );
}

#[test]
fn o103_scoped_call_withdraws_later_callee_changes_and_unknown_entry() {
    for source in [
        "proc answer {} {return 42}; proc f {} {set y [answer]}; rename answer saved; proc answer {} {return 99}",
        "proc answer {} {return 42}; proc f {} {set y [answer]}; unknown_future_entry",
        "proc answer {} {return 42}; proc f {} {set y [answer]}; trace add execution answer enter callback",
    ] {
        let opts = run_pass_with_command_mutations(source);
        assert!(
            opts.iter()
                .all(|optimisation| optimisation.code != DiagCode::O103 || optimisation.hint_only),
            "{source}: {opts:?}"
        );
    }
}

#[test]
fn o103_bare_call_stays_hint_only() {
    // Top-level `::answer` — the call result is discarded, so
    // folding it as a statement would leave an invalid `42`
    // command. Diagnostic must stay hint-only.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu =
        CompilationUnit::build_for("proc ::answer {} { return 42 }\n::answer", &registry, false)
            .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(!o103s.is_empty(), "expected at least one O103");
    assert!(
        o103s.iter().all(|o| o.hint_only),
        "bare-call form must stay hint-only, got {o103s:?}",
    );
}

#[test]
fn o103_folds_arg_sensitive_passthrough_cmd_subst() {
    // `::not_const {x} { return $x }` has no argument-*independent*
    // constant return, but with the call's constant arg bound it folds:
    // `[::not_const 1]` → `1` (the passthrough folds). The applicable
    // rewrite fires only for the CMD-subst form; the bare-call form
    // stays hint-only.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::not_const {x} { return $x }\nputs [::not_const 1]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "1" && !o.hint_only),
        "expected applicable O103 folding [::not_const 1] to 1, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_arg_sensitive_passthrough_with_braced_multiword_literal() {
    // `parse_static_call_args` shares `literal_words`'s proper Tcl-aware
    // tokeniser rather than a naive `split_whitespace`, so a braced
    // multi-word call argument (`{a b}` — one clean literal argument in
    // real Tcl) folds too.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::passthrough {x} { return $x }\nputs [::passthrough {a b}]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "{a b}" && !o.hint_only),
        "expected applicable O103 folding [::passthrough {{a b}}] to {{a b}}, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_variadic_args_proc_returning_args_directly() {
    // TP: `args` is seeded as a canonical Tcl list (see
    // `seed_params_from_args`), not skipped outright — `proc ::foo {a
    // args} { return $args }` called as `[::foo 1 2]` binds `a=1`,
    // `args={2}` (a one-element list, which renders bare as `2` with
    // no braces needed), so `return $args` folds to `2`. Confirmed
    // against tclsh 8.6: `puts [::foo 1 2]` prints `2`.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::foo {a args} { return $args }\nputs [::foo 1 2]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "2" && !o.hint_only),
        "expected applicable O103 folding [::foo 1 2] to 2 (args seeded as a list), got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_does_not_fold_recursive_or_loop_proc() {
    // A proc whose return SCCP cannot determine under the bound args must
    // not fold — a loop-carried `return $total` is a phi (Overdefined at
    // exit), even though a pre-loop `set total 0` leaves a stale Const(0).
    // Mis-folding it would be a miscompile.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc ::sum {ns} {\n  set total 0\n  foreach n $ns { set total [expr {$total + $n}] }\n  return $total\n}\nputs [::sum {1 2 3}]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "loop-carried return must not fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_implicit_return_proc_cmd_subst() {
    // TP: the KCS O103 doc's own canonical example — a proc with NO
    // explicit `return`, whose result is Tcl's "value of the last
    // command executed" (a bare `expr {…}`) — must fold. Before the
    // fall-through fix, `resolve_return_constant` only looked at
    // `Terminator::Return` blocks, so a pure proc that relies entirely
    // on implicit return never folded at all (`[double 21]` stayed
    // unrewritten even though tclsh evaluates it to `42`).
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc double {n} { expr {$n * 2} }\nset x [double 21]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
        "expected applicable O103 folding [double 21] to 42 (implicit return), got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_implicit_return_zero_arg_proc() {
    // TP: a zero-parameter implicit-return proc (`set` as the trailing
    // statement — Tcl's `set` returns the value it just assigned, so
    // this is exactly like `return $pi`) also folds via the CMD-subst
    // path.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc pi {} { set p 3 }\nputs [pi]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "3" && !o.hint_only),
        "expected applicable O103 folding [pi] to 3 (implicit `set` return), got {:?}; {}",
        ctx.optimisations,
        proc_fold_context(&cu, "::pi"),
    );
}

#[test]
fn o103_does_not_fold_when_return_and_fallthrough_disagree() {
    // FN / miscompile-guard: `helper`'s result is opaque to intraprocedural
    // SCCP (it's a call, not folded interprocedurally), so `g`'s `if`
    // condition can't be resolved even after `n` is seeded to 5 — both the
    // `return 1` branch AND the fall-through `expr {99}` stay executable.
    // tclsh 9.0.4 ground truth: `g 5` calls `helper 5` (== 6), `6 > 100` is
    // false, so `g 5` falls through and returns `99` — NOT `1`. Before the
    // fall-through fix, the return reading silently ignored the
    // reachable non-`Return` exit and confidently (wrongly) folded to the
    // `return 1` branch's value — a real miscompile, not just an
    // over-eager fold.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc helper {n} { expr {$n + 1} }\nproc g {n} {\n    if {[helper $n] > 100} {\n        return 1\n    }\n    expr {99}\n}\nputs [g 5]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "disagreeing return/fall-through exits must not produce an applicable fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_does_not_fold_through_empty_if_body() {
    // FP guard / miscompile-guard (reported in code review): a trailing
    // `if {$c} {}` with an empty body (and no `else`) is itself a real
    // Tcl command whose result is `""` — whether the condition is true
    // (the empty body's own result) or false (no branch ran, no
    // `else`). tclsh 9.0.4 ground truth: `f 0` and `f 1` both return
    // `""`, NOT `1` from the preceding `set x 1`. An earlier, more
    // permissive fall-through resolver walked straight through the
    // empty `if`-body block back to `set x 1` and wrongly folded `[f
    // 0]` to `1` — treating "0 statements" as "no command ran here"
    // rather than "this command's own result is empty". The fold must
    // decline entirely rather than inherit a stale prior value.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc f {c} { set x 1\nif {$c} {} }\nputs [f 0]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "empty if-body fall-through must not produce an applicable fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_variadic_args_proc_ignoring_trailing_args() {
    // TP: a proc with a trailing `args` parameter folds on its fixed
    // parameter when the call supplies MORE arguments than the proc has
    // fixed params — the extra args collect into `args`, unused by this
    // body. Before the `args` fix, `evaluate_proc_with_constants`
    // required `params.len() == args.len()` exactly, so a call
    // supplying more than one trailing argument (`[f 5 x y z]` for
    // `proc f {a args}`) never even reached the fold attempt.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc f {a args} { expr {$a * 2} }\nputs [f 5 x y z]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "10" && !o.hint_only),
        "expected applicable O103 folding [f 5 x y z] to 10, got {:?}",
        ctx.optimisations,
    );
}

/// Run the propagation pass with `ctx.command_mutations` populated —
/// the piece `optimise_raw` (used by the O129 trust-gate test) never
/// wires (it doesn't set `cu.interproc` either, so it can't drive
/// O103 at all) and a bare `PassContext::new` + `run` leaves at its
/// all-trusting `Default`. Mirrors the whole-module scan every real
/// production entry point (`optimise_unit_raw`, `find_dead_stores`,
/// `optimise_by_pass`) performs before running passes.
fn run_pass_with_command_mutations(source: &str) -> Vec<Optimisation> {
    let registry = CommandRegistry::build_default();
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &registry);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn o103_does_not_fold_call_to_proc_renamed_over() {
    // The selected moved allocation cannot borrow the replaced body's summary.
    // Original C9.0.4: double 21 returns 63 after this rename.
    let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\nputs [double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
        "call to a proc name later `rename`d over must not fold, got {opts:?}",
    );
}

#[test]
fn o103_folds_the_actual_procedure_selected_by_interp_alias() {
    // The selected alias target returns 99 on C Tcl 8.4–9.1.
    let src = "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\nputs [answer]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O103 && !o.hint_only && o.replacement == "99"),
        "the exact alias target must fold to its own result, got {opts:?}",
    );
    let jim = crate::optimiser::optimise_raw(src, &registry(), Some("jim"));
    assert!(jim.iter().all(|o| o.code != DiagCode::O103 || o.hint_only));
}

#[test]
fn o103_still_folds_unrelated_proc_despite_unrelated_rename_elsewhere() {
    // TN control: `trusts_proc_binding` must be per-name, not a blanket
    // "any rename anywhere disables all O103 folds" — an unrelated
    // rename of a *different* proc must not block folding calls to
    // procs it never touched.
    let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\nputs [double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
        "unrelated proc must still fold despite an unrelated rename, got {opts:?}",
    );
}

#[test]
fn o103_does_not_fold_call_to_proc_renamed_over_via_namespace_relative_rename() {
    // FP guard / miscompile-guard (reported in code review): `rename
    // triple double` inside `proc ::ns::doit` resolves both bare names
    // relative to `::ns` (the proc's own namespace) — it renames
    // `::ns::triple` onto `::ns::double`, NOT the (nonexistent)
    // top-level `::triple`/`::double`. tclsh 9.0.4 ground truth: after
    // `::ns::doit` runs, `::ns::double 21` executes `triple`'s body
    // (21*3 == 63), not `double`'s original body (21*2 == 42). An
    // earlier version of `collect_proc_rebindings` always rooted a bare
    // rename argument at the GLOBAL namespace (`::double`/`::triple`),
    // so it never marked `::ns::double`/`::ns::triple` as rebound and
    // O103 could fold `[::ns::double 21]` to the wrong `42`.
    let src = "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n::ns::doit\nputs [::ns::double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
        "call to a proc renamed over via a namespace-relative rename must not fold, got {opts:?}",
    );
}

#[test]
fn parse_cmd_subst_head_rejects_metachars() {
    assert_eq!(parse_cmd_subst_head("::answer"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head("::answer 1 2"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head("  ::answer"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head(""), None);
    // Leading `$` / `[` / `{` / `"` → would re-substitute.
    assert_eq!(parse_cmd_subst_head("$cmd"), None);
    assert_eq!(parse_cmd_subst_head("[cmd]"), None);
}

#[test]
fn o115_collapses_redundant_nested_expr_in_value_positions() {
    // A redundant double-`expr` cmd-sub
    // `[expr {[expr {E}]}]` collapses to `[expr {E}]` in command-arg
    // and return value positions (O115 otherwise fires only on a
    // standalone `expr` statement).
    let collapsed = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O115)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        collapsed("proc ::f {x} { puts [expr {[expr {$x * 2}]}] }"),
        vec!["[expr {$x * 2}]".to_string()],
        "command-arg position should collapse",
    );
    assert_eq!(
        collapsed("proc ::f {x} { return [expr {[expr {$x * 2}]}] }"),
        vec!["return [expr {$x * 2}]".to_string()],
        "return position should collapse",
    );
}

#[test]
fn o115_value_position_is_sound() {
    // A single `[expr {$x + 1}]` must NOT collapse (its unwrap
    // `$x + 1` would be invalid as a bare value), and an
    // `[expr {[other]}]` whose inner is not itself `expr` is not
    // value-equivalent for non-numeric results.
    let has_o115 = |src: &str| -> bool {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .iter()
            .any(|o| o.code == DiagCode::O115)
    };
    assert!(!has_o115("proc ::f {x} { puts [expr {$x + 1}] }"));
    assert!(!has_o115("proc ::f {x} { return [expr {$x + 1}] }"));
    assert!(!has_o115("proc ::f {x} { puts [expr {[someproc]}] }"));
}

fn fold(src: &str) -> Vec<String> {
    crate::optimiser::optimise_raw(src, &registry(), None)
        .into_iter()
        .filter(|o| o.code == DiagCode::O129)
        .map(|o| o.replacement)
        .collect()
}

fn profiled_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
}

fn fold_code(src: &str, code: &str) -> Vec<String> {
    crate::optimiser::optimise_raw(src, &registry(), None)
        .into_iter()
        .filter(|o| o.code.as_str() == code)
        .map(|o| o.replacement)
        .collect()
}

/// #2430: `string cat` arrives in 8.6 and `string reverse` in 8.5;
/// tclsh 8.4.20 / 8.5.19 raise `bad option` for the missing one, so the
/// fold declines under those releases and takes the value from 8.6 on.
#[test]
fn o129_declines_a_string_subcommand_the_release_lacks() {
    let folds = |release: &str, src: &str| -> Vec<String> {
        let context = tcl_registry::model::ingress::static_context_for(release);
        let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
        crate::optimiser::manager::optimise_with_dialect(src, context.commands(), Some(profile))
            .into_iter()
            .filter(|o| o.code == DiagCode::O129 && !o.hint_only)
            .map(|o| o.replacement)
            .collect()
    };
    let cat = "puts [string cat a b c]";
    let reverse = "puts [string reverse abc]";
    assert_eq!(folds("tcl8.4", cat), Vec::<String>::new());
    assert_eq!(folds("tcl8.5", cat), Vec::<String>::new());
    assert_eq!(folds("tcl8.4", reverse), Vec::<String>::new());
    assert_eq!(folds("tcl8.5", reverse), vec!["cba".to_string()]);
    assert_eq!(folds("tcl8.6", cat), vec!["abc".to_string()]);
}

#[test]
fn o129_folds_pure_builtin_cmd_subst() {
    // A pure-builtin cmd-sub with constant
    // (literal) args folds via the registry `const_fold` callback
    // (O129). `optimise_raw` sets `ctx.registry`, which the O129
    // path requires.
    assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
    assert_eq!(fold("puts [string tolower BAR]"), vec!["bar".to_string()]);
    assert_eq!(fold("puts [string reverse abc]"), vec!["cba".to_string()]);
    // The headline O129 example.
    assert_eq!(fold("puts [string length abcde]"), vec!["5".to_string()]);
    // The cat / repeat / trim folds.
    assert_eq!(
        fold("puts [string cat foo bar]"),
        vec!["foobar".to_string()]
    );
    assert_eq!(
        fold("puts [string repeat ab 3]"),
        vec!["ababab".to_string()]
    );
    assert_eq!(fold("puts [string trim {  hi  }]"), vec!["hi".to_string()]);
    assert_eq!(
        fold("puts [string range abcde 1 3]"),
        vec!["bcd".to_string()]
    );
    assert_eq!(fold("puts [string index abc end]"), vec!["c".to_string()]);
    // List + dict folds.
    assert_eq!(fold("puts [llength {a b c}]"), vec!["3".to_string()]);
    // The cmd-sub replacement is one word, so a spaced result is
    // brace-quoted by `render_propagation_word`.
    assert_eq!(fold("puts [concat a b c]"), vec!["{a b c}".to_string()]);
    assert_eq!(fold("puts [join {a b c} -]"), vec!["a-b-c".to_string()]);
    // `lindex` / `list` report their own codes (O118 / O116) for
    // editor granularity, so filter by those rather than O129.
    assert_eq!(
        fold_code("puts [lindex {a b c} 1]", "O118"),
        vec!["b".to_string()]
    );
    assert_eq!(fold_code("set x [list]", "O116"), vec!["{}".to_string()]);
    // A list 8.6 would read apart as a script, once a literal, stays the
    // list (`if 1 $x` ranges nothing; the braced literal ranges `abc`).
    let hazard = "set x [list string range abc 0 end-2147483649]\nif 1 $x";
    let o116 = |dialect| {
        crate::optimiser::optimise_raw(hazard, &registry(), Some(dialect))
            .into_iter()
            .filter(|o| o.code.as_str() == "O116")
            .count()
    };
    assert_eq!(o116("tcl8.6"), 0);
    assert_eq!(o116("tcl9.0"), 1);
    assert_eq!(fold("puts [dict get {a 1 b 2} b]"), vec!["2".to_string()]);
    assert_eq!(
        fold("puts [lreverse {a b c}]"),
        vec!["{c b a}".to_string()],
        "list result is brace-quoted as one word",
    );
    assert_eq!(fold("puts [string map {a b} aaa]"), vec!["bbb".to_string()]);
    assert_eq!(fold("puts [subst hello]"), vec!["hello".to_string()]);
    // `subst` with a substitution must NOT fold (no upstream resolution).
    assert_eq!(fold("puts [subst {$x}]"), [] as [std::string::String; 0]);
    // `string is` Tcl-faithful classes.
    assert_eq!(fold("puts [string is alpha abc]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is lower abc1]"), vec!["0".to_string()]);
    assert_eq!(fold("puts [string is boolean yes]"), vec!["1".to_string()]);
    // The convenience driver retains its documented C9 native target.
    // Actual C8.4 and Jim reject these classes; the selected target matters.
    assert_eq!(fold("puts [string is list {a b c}]"), vec!["1".to_owned()]);
    let legacy = tcl_registry::model::ingress::static_context_for("tcl8.4");
    for source in [
        "puts [string is list {a b c}]",
        "puts [string is wideinteger 42]",
    ] {
        assert!(
            crate::optimiser::optimise_raw_for_profile(
                source,
                legacy.commands(),
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile()
                ),
            )
            .iter()
            .all(|finding| finding.code != DiagCode::O129)
        );
    }
    // `format` %s / %d / %% subset.
    assert_eq!(fold("puts [format %d 42]"), vec!["42".to_string()]);
    assert_eq!(fold("puts [format {v=%s} hi]"), vec!["v=hi".to_string()]);
    // `format` flag / width / precision folds for
    // the decimal-integer + string conversions (dialect-invariant).
    assert_eq!(fold("puts [format %05d 7]"), vec!["00007".to_string()]);
    assert_eq!(fold("puts [format %.3d 5]"), vec!["005".to_string()]);
    // `%#d` stays unfolded (`0d5` on Tcl 9, `5` on 8.6 — divergent).
    assert_eq!(fold("puts [format %#d 5]"), [] as [std::string::String; 0]);
    // `string is integer` / `double` fold over their
    // dialect-invariant subsets.
    assert_eq!(fold("puts [string is integer 42]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is double 1.5]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is double abc]"), vec!["0".to_string()]);
    assert_eq!(
        fold("puts [string is wideinteger 42]"),
        vec!["1".to_owned()]
    );
    // A braced literal with a space → result rendered as one word.
    assert_eq!(
        fold("puts [string toupper {a b}]"),
        vec!["{A B}".to_string()],
    );
    // A `$var` arg is not a constant literal → no fold.
    assert_eq!(
        fold("puts [string toupper $x]"),
        [] as [std::string::String; 0]
    );
    // The range form (`string toupper s first last`) does not fold.
    assert_eq!(
        fold("puts [string toupper foo 0 0]"),
        [] as [std::string::String; 0]
    );
    // A non-builtin head (a user proc) is not an O129 candidate.
    assert_eq!(
        fold("proc ::p {} { return 1 }\nputs [::p]"),
        [] as [std::string::String; 0]
    );
}

/// The folded command substitution carries its own quoted argument, so
/// the rewrite span has to cross that inner `"b"`. A scanner that stops
/// at the first unescaped `"` produces a span covering only
/// `"a[string toupper "`, and applying the fix leaves the
/// tail `b"]c"` behind as a stray fragment. Applying the rewrite must
/// yield well-formed source.
#[test]
fn o129_interpolation_fold_replaces_the_whole_quoted_word() {
    let src = "puts \"a[string toupper \"b\"]c\"\n";
    let opts = crate::optimiser::optimise_raw(src, &registry(), None);
    let fold = opts
        .iter()
        .find(|o| o.code == DiagCode::O129 && !o.hint_only)
        .unwrap_or_else(|| panic!("expected an applicable O129, got {opts:?}"));
    let mut rewritten = src.to_owned();
    rewritten.replace_range(
        fold.span.start() as usize..fold.span.end() as usize,
        &fold.replacement,
    );
    assert_eq!(rewritten, "puts \"aBc\"\n");
}

#[test]
fn retained_const_subst_accepts_only_closed_literal_templates() {
    let literal = "puts [string toupper \"a\\tb\"]";
    let findings = crate::optimiser::optimise_raw(literal, &registry(), None);
    let fold = findings
        .iter()
        .find(|finding| finding.code == DiagCode::O129 && !finding.hint_only)
        .expect("the native word owner resolves a quoted literal");
    assert_eq!(
        tcl_syntax::list::split_list(&fold.replacement).unwrap(),
        vec!["A\tB"]
    );
    for source in [
        "proc p {value} {puts [string toupper \"$value\"]}",
        "puts [string toupper \"[operation]\"]",
    ] {
        assert!(
            crate::optimiser::optimise_raw(source, &registry(), None)
                .iter()
                .all(|finding| finding.code != DiagCode::O129 || finding.hint_only),
            "{source}"
        );
    }
}

/// A `"…"` word holding a command-position comment reaches across a
/// backslash-newline continuation. `tclsh` prints `x3abc`; a rewrite that
/// mis-spans the word prints `x3ab""b"c`.
///
/// Both the canonical lexer and `close_quote_offset` keep command
/// position across the continuation and reach the final `"`, so the safe
/// rewrite folds only the first command substitution and preserves the
/// commented-out closer verbatim.
#[test]
fn o129_fold_preserves_comment_hidden_closer() {
    let src = "puts \"x[string length abc]a[\n\\\n# ] comment\nset y \"b\"\n]c\"\n";
    let opts = crate::optimiser::optimise_raw(src, &registry(), None);
    let rewrites = opts
        .iter()
        .filter(|o| o.code == DiagCode::O129 && !o.hint_only)
        .collect::<Vec<_>>();
    assert_eq!(rewrites.len(), 1, "expected one safe O129: {opts:?}");
    let opt = rewrites[0];
    let mut rewritten = src.to_owned();
    rewritten.replace_range(
        opt.span.start() as usize..opt.span.end() as usize,
        &opt.replacement,
    );
    assert_eq!(
        rewritten,
        "puts \"x3a[\n\\\n# ] comment\nset y \"b\"\n]c\"\n"
    );
}

#[test]
fn o129_concat_preserves_native_trailing_backslash_space_bytes() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    // All six native engines re-expose exactly one trailing byte after a
    // backslash. The shared concat owner now models that operation.
    for (source, expected) in [
        ("puts [concat a b c]", "a b c"),
        ("puts [concat a {b\\ } c]", "a b\\  c"),
        ("puts [concat {b\\ }]", "b\\ "),
        ("puts [concat a {b\\\\   } c]", "a b\\\\  c"),
    ] {
        let replacements = crate::optimiser::optimise_raw(source, profiled_registry(), None)
            .into_iter()
            .filter(|finding| finding.code == DiagCode::O129 && !finding.hint_only)
            .map(|finding| finding.replacement)
            .collect::<Vec<_>>();
        assert_eq!(replacements.len(), 1, "{source}");
        assert_eq!(
            tcl_syntax::list::split_list(&replacements[0]).unwrap(),
            vec![expected],
            "{source}"
        );
    }
}

#[test]
fn o129_resolves_constant_var_args_b2() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    // A constant `$var` arg in a builtin
    // cmd-sub is resolved (whole-function-constant → sound) before
    // folding; a multi-word value stays a single argument.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, profiled_registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        fold("set s abcde\nputs [string length $s]"),
        vec!["5".to_string()],
    );
    // A multi-word constant value is kept as one arg.
    assert_eq!(
        fold("set s {a b}\nputs [llength $s]"),
        vec!["2".to_string()],
    );
    // Combined: a resolved var inside an interpolation cmd-sub.
    assert_eq!(
        fold("set s abc\nputs \"len=[string length $s]\""),
        vec!["\"len=3\"".to_string()],
    );
    // A non-constant var does not resolve → no fold.
    assert_eq!(
        fold("puts [string length $undefined]"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn o129_folds_embedded_cmd_subst_in_interpolation() {
    // A pure-builtin cmd-sub embedded inside
    // an interpolation string folds, splicing the raw result.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        fold("puts \"v=[string length abc]\""),
        vec!["\"v=3\"".to_string()],
    );
    assert_eq!(
        fold("puts \"n=[llength {a b c}] items\""),
        vec!["\"n=3 items\"".to_string()],
    );
    // A non-foldable embedded sub leaves the string unfolded.
    assert_eq!(
        fold("puts \"x=[someproc]\""),
        [] as [std::string::String; 0]
    );
    // Soundness guard: a result that would re-introduce a `$`
    // substitution into the `\"…\"` is not spliced.
    assert_eq!(
        fold("puts \"v=[string cat {$} x]\""),
        [] as [std::string::String; 0]
    );
}

#[test]
fn o129_uses_the_actual_binding_before_and_after_a_rebinder() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, profiled_registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    // Baseline: untouched `string` folds.
    assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
    // An uncalled declaration leaves the stock command selected.
    assert_eq!(
        fold("proc clobber {} { rename string {} }\nputs [string toupper foo]"),
        vec!["FOO".to_string()],
    );
    assert_eq!(
        fold("proc clobber {} { rename string {} }\nclobber\nputs [string toupper foo]"),
        [] as [String; 0],
        "a completed rebinder removes the reached builtin",
    );
    assert_eq!(
        fold("rename string {}\nputs [string toupper foo]").len(),
        0,
        "renamed-away builtin must not fold",
    );
}

#[test]
fn run_passes_dispatches_propagation() {
    let reg = registry();
    let cu = CompilationUnit::build_for("set x 9\nputs $x", &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.registry = Some(&reg);
    super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Propagation]);
    assert!(
        ctx.optimisations.iter().any(|o| o.code == DiagCode::O100),
        "expected O100 via run_passes, got {:?}",
        ctx.optimisations,
    );
}
#[test]
fn retained_substitution_folds_use_the_selected_namespace_path_procedure() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "proc ::p {} {return GLOBAL}\n\
                      namespace eval ::lib {proc p {} {return PATH}}\n\
                      namespace eval ::N {namespace path ::lib; proc caller {} {puts [p]}}\n\
                      ::N::caller";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0")
        .with_interprocedural(
            registry,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()),
        );
    let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    context.registry = Some(registry);
    run(&mut context, &cu);
    assert!(
        context.optimisations.iter().all(|optimisation| {
            optimisation.code != DiagCode::O103
                || optimisation.hint_only
                || optimisation.replacement != "GLOBAL"
        }),
        "{:?}",
        context.optimisations
    );
}

#[test]
fn retained_substitution_folds_decline_namespaced_builtin_shadows() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for invocation in ["[expr {1 + 2}]", "[string length abc]", "[list a b]"] {
        let source = format!(
            "namespace eval N {{proc expr {{args}} {{return EXPR}}; proc string {{args}} {{return STRING}}; proc list {{args}} {{return LIST}}; proc run {{}} {{puts {invocation}}}}}\nN::run"
        );
        let cu = CompilationUnit::build_for_dialect(&source, registry, false, "tcl9.0")
            .with_interprocedural(
                registry,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                ),
            );
        let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        context.registry = Some(registry);
        run(&mut context, &cu);
        assert!(
            context.optimisations.iter().all(|optimisation| {
                !matches!(
                    optimisation.code,
                    DiagCode::O101
                        | DiagCode::O115
                        | DiagCode::O116
                        | DiagCode::O118
                        | DiagCode::O129
                ) || optimisation.hint_only
                    || !source
                        .get(optimisation.span.as_range())
                        .is_some_and(|text| text.contains(invocation))
            }),
            "{source}: {:?}",
            context.optimisations
        );
    }
}

#[test]
fn retained_substitution_missing_child_receipt_withdraws_applicable_folding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "puts [list a b]";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0");
    let statement = &cu.ir_module.top_level.statements[0];
    let mut tokens = cu
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .expect("original carrier")
        .clone();
    tokens.source_binding = None;
    let mut context = PassContext::new(source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    visit_call_cmd_subst_folds(
        &mut context,
        &cu,
        &tokens,
        &std::collections::HashMap::new(),
        "::",
        statement.span(),
    );
    assert_eq!(context.optimisations.len(), 0);
}
#[test]
fn retained_substitution_procedure_folding_preserves_arity_and_argument_effects() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in [
        "proc f {required} {return CONSTANT}; puts [f]",
        "proc f {} {return CONSTANT}; puts [f EXTRA]",
        "proc f {ignored} {return CONSTANT}; set x 0; puts [f [incr x]]; puts $x",
    ] {
        let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0")
            .with_interprocedural(registry, registry.profile());
        let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        context.registry = Some(registry);
        run(&mut context, &cu);
        assert!(
            context
                .optimisations
                .iter()
                .all(|optimisation| optimisation.code != DiagCode::O103 || optimisation.hint_only),
            "{source}: {:?}",
            context.optimisations
        );
    }
}

/// A literal a write the statements do not show may have replaced is
/// never forwarded: an arm of a `switch` the flow graph keeps as one
/// statement, a callback script, or a command the module cannot see.
#[test]
fn o100_and_o102_never_forward_a_literal_a_hidden_write_may_have_replaced() {
    for (src, stale) in [
        (
            "set go 1\nset s q1\nswitch -glob -- $s { q* { set go 0 } }\nputs $go",
            "1",
        ),
        (
            "set go 1\nswitch -nocase -- [gets stdin] { q { set go 0 } }\nputs $go",
            "1",
        ),
        (
            "proc p {s} {\n set go 1\n switch -glob -- $s { q* { set go 0 } }\n puts $go\n}",
            "1",
        ),
        (
            "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nputs $go",
            "1",
        ),
        (
            "set done 0\nafter 10 { set done 1 }\nafter 50\nupdate\nputs $done",
            "0",
        ),
        (
            "proc tick {} { set ::done 1 }\nset done 0\nafter 10 tick\nafter 50\nupdate\nputs $done",
            "0",
        ),
        ("set g 5\nfoo\nputs $g", "5"),
        ("set ::g 5\nfoo\nputs $::g", "5"),
        ("proc p {} {\n set g 5\n foo\n puts $g\n}", "5"),
        ("set g 5\nsource other.tcl\nputs $g", "5"),
        ("proc p {} {\n set g 5\n source other.tcl\n puts $g\n}", "5"),
        (
            "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set line 5\n switch -glob -- $s { g* { zero line } }\n puts $line\n}",
            "5",
        ),
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\nputs $go",
            "1",
        ),
    ] {
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == stale)),
            "{src}: {opts:?}"
        );
    }
}

#[test]
fn retained_substitution_native_object_callbacks_withdraw_builtin_erasure() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let source = "proc f {input} {puts [llength $input]}";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.1");
    let statement = &cu
        .ir_module
        .procedures
        .get(&fixture_id(&cu.ir_module.procedures, "::f"))
        .expect("authored fixture body")
        .body
        .statements[0];
    let tokens = cu
        .ir_module
        .procedures
        .get(&fixture_id(&cu.ir_module.procedures, "::f"))
        .expect("authored fixture body")
        .body
        .retained_source_tokens_for_statement(statement)
        .expect("original carrier");
    let calls =
        retained_substitution_calls(tokens, &cu.ir_module, registry).expect("complete children");
    let call = calls
        .iter()
        .find(|call| {
            call.tokens.as_ref().is_some_and(|tokens| {
                tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "llength")
            })
        })
        .expect("original length invocation");
    let metadata = Some(cu.top_level.invocation_metadata_context(registry).unwrap());
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        metadata,
        call.tokens.as_ref().unwrap(),
    )
    .expect("selected builtin independent of native object effects");
    assert!(invocation.facts.effects.requires_world_barrier());
    assert!(
        fold_retained_builtin(
            registry,
            call,
            &calls,
            None,
            metadata,
            (
                None,
                None,
                tcl_registry::value_transfer::ScriptEncoding::Auto
            )
        )
        .is_none()
    );
}

#[test]
fn retained_substitution_formal_seeds_share_defaults_and_jim_rest_names() {
    use tcl_dialect::ParameterGrammar::{Jim, Tcl};
    let policy = crate::tcl_expr_eval::FoldPolicy::default();
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("{value DEFAULT} args", Tcl).unwrap();
    let seed = seed_params_from_args(&parameters, &[], Tcl, policy).unwrap();
    assert_eq!(
        seed.get(&("value".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String("DEFAULT".into())))
    );
    assert_eq!(
        seed.get(&("args".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String(String::new())))
    );
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("{args remaining} tail", Jim)
            .unwrap();
    let seed = seed_params_from_args(
        &parameters,
        &[ConstValue::Int(1), ConstValue::Int(2), ConstValue::Int(3)],
        Jim,
        policy,
    )
    .unwrap();
    assert_eq!(
        seed.get(&("remaining".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String("1 2".into())))
    );
    assert_eq!(
        seed.get(&("tail".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::Int(3)))
    );
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("&reference", Jim).unwrap();
    assert!(
        seed_params_from_args(&parameters, &[ConstValue::String("x".into())], Jim, policy)
            .is_none()
    );
}
#[test]
fn retained_expression_argument_folding_requires_actual_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual original registrations are retained independently of metadata
    // availability. The owner stays alive through every refusal control.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut catalogue = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut descriptor = catalogue.get("expr").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    catalogue.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)));
    let registry = current.commands();
    let (_owner, native) =
        crate::environment_ingress::captured_native_entry_with_owner(registry.profile().unwrap());
    let entry = crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(std::sync::Arc::new(native)),
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let mut unit = CompilationUnit::build_with_context_registry(
        "puts [expr {1 + 2}]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: registry.profile(),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    let statement = &unit.ir_module.top_level.statements[0];
    let tokens = unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap()
        .clone();
    let calls = retained_substitution_calls(&tokens, &unit.ir_module, registry).unwrap();
    let call = calls.iter().find(|call| call.command == "expr").unwrap();
    let constants = std::collections::HashMap::new();
    let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    assert_eq!(
        try_o101_expr_arg_fold(&context, &unit, &tokens, call, &constants),
        Some("3".into())
    );
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for withheld in [Some(older), Some(foreign), None] {
        unit.ir_module.source_metadata_input = withheld.map(|availability| {
            crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                availability,
                input.lexer_config(),
            )
        });
        let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
        context.registry = Some(registry);
        assert!(try_o101_expr_arg_fold(&context, &unit, &tokens, call, &constants).is_none());
    }
}

#[test]
fn retained_recursive_folding_preserves_actual_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let cu = CompilationUnit::build_for_dialect(
        "puts [list [dict create key value]]",
        registry,
        false,
        "tcl8.6",
    );
    let statement = &cu.ir_module.top_level.statements[0];
    let tokens = cu
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let calls = retained_substitution_calls(tokens, &cu.ir_module, registry).unwrap();
    let outer = calls.iter().find(|call| call.command == "list").unwrap();
    assert_eq!(
        fold_retained_builtin(
            registry,
            outer,
            &calls,
            None,
            Some(current.into()),
            (
                None,
                None,
                tcl_registry::value_transfer::ScriptEncoding::Auto
            )
        ),
        Some("{key value}".into())
    );
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for metadata in [Some((&older).into()), Some(foreign.into()), None] {
        assert!(
            fold_retained_builtin(
                registry,
                outer,
                &calls,
                None,
                metadata,
                (
                    None,
                    None,
                    tcl_registry::value_transfer::ScriptEncoding::Auto
                )
            )
            .is_none()
        );
    }
    let tokens = tokens.clone();
    let span = statement.span();
    let mut missing = cu;
    missing.ir_module.source_metadata_input = None;
    let mut context = PassContext::new(&missing.source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    visit_call_cmd_subst_folds(
        &mut context,
        &missing,
        &tokens,
        &std::collections::HashMap::new(),
        "::",
        span,
    );
    assert_eq!(context.optimisations.len(), 0);
}
/// The controls: a name the hidden writes never name, a name assigned
/// after the call the module cannot see, and a read the call follows still
/// forward.
#[test]
fn o100_and_o102_still_forward_a_literal_no_hidden_write_can_replace() {
    for (src, literal) in [
        ("proc p {} {\n set g 5\n puts $g\n foo\n}", "5"),
        (
            "set go 1\nswitch -glob -- [gets stdin] { q* { set other 0 } }\nputs $go",
            "1",
        ),
        ("foo\nset g 5\nputs $g", "5"),
        (
            "proc hit {} {global hits; incr hits}\nproc p {s} {\n set n 5\n switch -glob -- $s { g* { hit } }\n puts $n\n}",
            "5",
        ),
    ] {
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .any(|o| matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == literal),
            "{src}: {opts:?}"
        );
    }
}
