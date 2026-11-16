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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

/// A Windows document: the cooked `namespace eval` body folds CRLF while
/// the lowered body unit keeps original offsets, so its span must cover
/// the whole written region up to the closing brace.
#[test]
fn crlf_namespace_eval_body_unit_spans_its_written_region() {
    let source = "namespace eval ::x {\r\n    set a 1\r\n    set b 2\r\n}\r\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let module = super::lower_to_ir_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        Some(tcl_dialect::DialectProfile::find("tcl8.6").expect("profile")),
    );
    let open = u32::try_from(source.find('{').unwrap() + 1).unwrap();
    let close = u32::try_from(source.rfind('}').unwrap()).unwrap();
    let spans = module
        .body_units
        .values()
        .filter(|unit| unit.body_kind == crate::ir::ProcedureBodyKind::NamespaceScript)
        .map(|unit| unit.span)
        .collect::<Vec<_>>();
    assert_eq!(spans, vec![tcl_lexer::Span::new(open, close)]);
}

fn fixture_method<'a>(
    methods: &'a crate::ir::MethodInventory,
    label: &str,
) -> Option<&'a crate::ir::MethodDef> {
    methods.unique_display(label).map(|(_, method)| method)
}

/// An opaque invocation retains its original evaluated argv without licensing
/// native instructions, a known normal store, or an invented entered body.
pub(super) fn assert_runtime_opaque_call(
    statement: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    expected_head: &str,
) {
    let crate::ir::Statement::Call {
        command,
        args,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        panic!("expected retained invocation, got {statement:?}");
    };
    assert_eq!(command, expected_head);
    assert_eq!(tokens.words().len(), args.len() + 1);
    assert!(tokens.source_binding.is_some(), "missing dispatch receipt");
    assert!(crate::registry_invocation::proved_native_inline_operation(tokens).is_none());
    if let Some(region) = tokens.evaluated_body() {
        assert!(
            region.possible_bodies.is_some(),
            "runtime retains an analysis-only body inventory"
        );
        assert_eq!(
            region.selection,
            crate::execution_region::RegionSelection::MaySkip
        );
        assert_eq!(
            region.residual_effects,
            crate::execution_region::WrapperEffectProjection::OpaqueResidual
        );
    }
    let context = &tokens.source_binding.as_ref().unwrap().variable_context;
    let mutations = crate::place_bridge::statement_mutation_places(statement, context, registry);
    assert!(
        mutations
            .iter()
            .any(|place| place.kind == crate::place::PlaceKind::Unknown)
            || tokens.evaluated_body().is_some()
            || crate::registry_invocation::normal_transfer_invocation(registry, None, tokens)
                .is_none(),
        "a runtime body cannot grant bounded normal stores: {mutations:?}"
    );
}

/// Every C-Tcl dialect folds a brace-word line continuation; the Jim
/// answer is asserted in `jim_brace_continuation_binds_different_names`.
const FOLD_RULES: WordValueRules = WordValueRules {
    brace: tcl_dialect::BraceBackslashNewline::Folds,
    list: tcl_dialect::ListParse::Strict,
};

/// `JimTcl`: the braced word keeps its bytes, and a malformed remainder is
/// split rather than raised.
const LITERAL_RULES: WordValueRules = WordValueRules {
    brace: tcl_dialect::BraceBackslashNewline::Literal,
    list: tcl_dialect::ListParse::Lenient,
};

/// A brace-word line continuation is folded to a space by every build of
/// the Tcl core and kept verbatim by `JimTcl`, so a wrapped parameter or
/// variable list binds *different names* in the two.
///
/// Measured on jimsh 0.84 against tclsh 8.6. The two word kinds resolve
/// the surviving `\<newline>` differently, because only one of them has a
/// `name ?default?` level:
///
/// * **Formal parameters** — `proc p {a b\<newline>c} {}` binds three
///   parameters under Tcl and *two* under Jim: the second element is the
///   specifier `b c`, so the parameter is `b` **defaulting to `c`**
///   (`p 1` leaves `b` as `c`; `p 1 2` sets it to `2`).
/// * **Variable lists** — `foreach {a b\<newline>c} …` has no such level,
///   so Jim binds a variable whose name really is `b c`
///   (`info exists {b c}` is 1 and `info exists b` is 0).
///
/// Jim's `info args` is no help here: unlike Tcl's it reports the raw
/// specifiers (`a {b c}`), not the parameter names.
#[test]
fn jim_brace_continuation_binds_different_names() {
    let wrapped = "a b\\\nc";

    assert_eq!(
        parse_formal_param_names(
            wrapped,
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"],
        "C Tcl folds the continuation into a separator"
    );
    assert_eq!(
        parse_formal_param_names(
            wrapped,
            LITERAL_RULES,
            Some(tcl_dialect::ParameterGrammar::Jim)
        )
        .unwrap(),
        vec!["a", "b"],
        "Jim keeps the bytes, so `b c` is one specifier: `b` defaulting to `c`"
    );

    assert_eq!(
        parse_var_list_names(wrapped, FOLD_RULES).unwrap(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        parse_var_list_names(wrapped, LITERAL_RULES).unwrap(),
        vec!["a", "b c"]
    );
}

/// `JimTcl`'s list parser never raises, so a word whose list text is
/// malformed still binds names instead of collapsing lowering to a
/// barrier. `llength "a {b"` is 2 under jimsh 0.84 and an error under
/// tclsh 8.6, and the same split backs `foreach`'s variable list.
#[test]
fn malformed_list_word_binds_names_only_for_jim() {
    assert_eq!(
        parse_var_list_names("a {b", FOLD_RULES),
        None,
        "C Tcl raises, which lowering turns into a barrier"
    );
    assert_eq!(
        parse_var_list_names("a {b", LITERAL_RULES).unwrap(),
        vec!["a", "b"],
        "Jim splits it anyway — the unterminated element runs to the end"
    );
}

/// Both axes reach the helpers from the dialect, so `list_parse` is not
/// silently left at its `Strict` default for a Jim document.
#[test]
fn list_word_rules_come_from_the_dialect() {
    use tcl_dialect::model::{Family, grammar};
    for &release in Family::Jim.releases() {
        let config = tcl_lexer::LexerConfig::from_grammar(grammar(Family::Jim, release));
        assert_eq!(
            WordValueRules::from_config(&config),
            LITERAL_RULES,
            "{release}"
        );
    }
    for &release in Family::Tcl.releases() {
        let config = tcl_lexer::LexerConfig::from_grammar(grammar(Family::Tcl, release));
        assert_eq!(
            WordValueRules::from_config(&config),
            FOLD_RULES,
            "{release}"
        );
    }
}

/// The knob is wired to the dialect, not chosen per call site.
#[test]
fn brace_continuation_comes_from_the_dialect() {
    use tcl_dialect::model::{Family, grammar};
    for &release in Family::Jim.releases() {
        assert_eq!(
            grammar(Family::Jim, release).brace_backslash_newline,
            tcl_dialect::BraceBackslashNewline::Literal,
            "{release}"
        );
    }
    for &release in Family::Tcl.releases() {
        assert_eq!(
            grammar(Family::Tcl, release).brace_backslash_newline,
            tcl_dialect::BraceBackslashNewline::Folds,
            "{release}"
        );
    }
}

use super::*;

fn reg() -> CommandRegistry {
    // These fixtures assert concrete C Tcl behaviour, including 9.1 grammar.
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

#[test]
fn borrowed_lowering_retains_exact_ingress_before_finalisation() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let profile = registry.profile().expect("selected profile");
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let mut lowerer = Lowerer::with_config(registry, config);
    let module = lowerer.lower("set x 003");
    assert!(std::ptr::eq(
        module.resolved_profile().expect("profile"),
        profile
    ));
    assert!(module.registry_snapshot.is_some());
    assert_eq!(module.lexer_config, config);
    assert_eq!(
        module.word_values(),
        tcl_syntax::word_rules::WordValueRules::from_config(&config)
    );
}

#[test]
fn runtime_procedure_target_retains_its_actual_activation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut lowerer = Lowerer::new(registry);
    let module = lowerer.lower_procedure_target("puts $x", "");
    let binding = module.top_level.statements[0]
        .tokens()
        .and_then(|tokens| tokens.source_binding.as_ref())
        .expect("positioned runtime procedure invocation");
    assert_eq!(
        binding.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "::top".to_owned(),
        }
    );
    assert_eq!(
        binding.variable_context.activation.as_deref(),
        Some("::top")
    );
}

#[test]
fn catch_options_specialisation_requires_the_selected_completion_grammar() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = format!("tcl{}", version.version_string());
        let registry = tcl_registry::model::ingress::static_context_for(&dialect).commands();
        let module = lower_to_ir("catch {set x executed} result options", registry);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert!(matches!(
                module.top_level.statements[0],
                Statement::Barrier { .. }
            ));
        } else {
            assert!(matches!(
                module.top_level.statements[0],
                Statement::Catch { .. }
            ));
        }
    }
}

#[test]
fn fatal_array_index_source_mask_follows_the_release() {
    let source = "puts $array({key})";
    for dialect in ["tcl8.4", "tcl8.6"] {
        assert_eq!(
            first_fatal_parse_error_with_config(
                source,
                tcl_lexer::LexerConfig::for_dialect(dialect)
            ),
            None,
            "{dialect} accepts the index text"
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        assert_eq!(
            first_fatal_parse_error_with_config(
                source,
                tcl_lexer::LexerConfig::for_dialect(dialect)
            ),
            Some(tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX.to_owned()),
            "{dialect} rejects the raw brace"
        );
    }
}

#[test]
fn irules_priority_uses_proved_native_identity_and_frozen_argument_value() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    for (source, expected) in [
        (
            "proc priority {value} {return}; priority 700; when HTTP_REQUEST {}",
            500,
        ),
        (
            "set selected 700; priority $selected; when HTTP_REQUEST {}",
            700,
        ),
    ] {
        let module = lower_to_ir_with_config(
            source,
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert_eq!(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST"))
                .expect("authored fixture body")
                .base_priority,
            expected
        );
    }
}

#[test]
fn irules_event_priorities_follow_file_state_and_keep_repeated_handlers() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "priority 700\n\
             when HTTP_REQUEST {}\n\
             priority 400\n\
             when HTTP_REQUEST {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    let priorities: Vec<_> = [
        "::when::HTTP_REQUEST",
        "::when::HTTP_REQUEST#1",
        "::when::HTTP_REQUEST#2",
        "::when::HTTP_REQUEST#3",
    ]
    .into_iter()
    .map(|name| {
        module
            .procedures
            .get(&fixture_id(&module.procedures, name))
            .expect("authored fixture body")
            .base_priority
    })
    .collect();
    assert_eq!(priorities, [700, 400, 200, 400]);
}

#[test]
fn irules_declaration_body_shape_gates_lowered_regions() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "when HTTP_REQUEST priority 1001 {}\n\
             when HTTP_REQUEST bare_body\n\
             when CLIENT_DATA \"quoted body\"\n\
             proc bare_proc {} bare_body\n\
             proc quoted_proc {} \"return\"\n\
             proc valid_proc {} { return }\n\
             when NOT_A_REAL_EVENT {}\n\
             when HTTP_REQUEST priority 200 {}\n\
             when HTTP_REQUEST {}",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    assert!(
        fixture_id_if_present(&module.procedures, "::when::NOT_A_REAL_EVENT").is_some(),
        "a syntactically valid unknown event is lowered so the XC translator can report it"
    );
    for name in ["::when::CLIENT_DATA", "::bare_proc", "::quoted_proc"] {
        assert!(
            fixture_id_if_present(&module.procedures, name).is_none(),
            "a non-braced declaration body must not open {name}"
        );
    }
    assert!(fixture_id_if_present(&module.procedures, "::valid_proc").is_some());
    let request = [
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST"))
            .expect("authored fixture body")
            .base_priority,
        module
            .procedures
            .get(&fixture_id(&module.procedures, "::when::HTTP_REQUEST#1"))
            .expect("authored fixture body")
            .base_priority,
    ];
    assert_eq!(request, [200, 500], "valid repeated handlers keep priority");
}

#[test]
fn irules_declarations_only_lower_at_the_file_surface() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let module = lower_to_ir_with_config(
        "if {1} {\n\
                 when CLIENT_DATA { pool top_level_nested_when }\n\
                 proc top_level_nested_proc {} { pool top_level_nested_proc }\n\
             }\n\
             when HTTP_REQUEST {\n\
                 if {1} {\n\
                     when SERVER_DATA { pool event_nested_when }\n\
                     proc event_nested_proc {} { pool event_nested_proc }\n\
                 }\n\
                 pool live_event\n\
             }\n\
             proc helper {} {\n\
                 if {1} {\n\
                     when HTTP_RESPONSE { pool proc_nested_when }\n\
                     proc proc_nested_proc {} { pool proc_nested_proc }\n\
                 }\n\
                 pool live_proc\n\
             }\n",
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );

    for absent in [
        "::when::CLIENT_DATA",
        "::when::SERVER_DATA",
        "::when::HTTP_RESPONSE",
        "::top_level_nested_proc",
        "::event_nested_proc",
        "::proc_nested_proc",
    ] {
        assert!(
            fixture_id_if_present(&module.procedures, absent).is_none(),
            "nested declaration must remain inert: {absent}"
        );
    }
    assert!(fixture_id_if_present(&module.procedures, "::when::HTTP_REQUEST").is_some());
    assert!(fixture_id_if_present(&module.procedures, "::helper").is_some());
}

#[test]
fn static_bool_accepts_unique_prefix_booleans() {
    // A condition folds boolean words by unique prefix, like Tcl
    // (`if {tr}` runs, `if {of}` does not — verified against tclsh).
    assert_eq!(static_bool("tr"), Some(true));
    assert_eq!(static_bool(" ye "), Some(true));
    assert_eq!(static_bool("on"), Some(true));
    assert_eq!(static_bool("of"), Some(false));
    assert_eq!(static_bool("n"), Some(false));
    assert_eq!(static_bool("0"), Some(false));
    assert_eq!(static_bool("1"), Some(true));
    // Ambiguous `o` (on/off) is not a static boolean.
    assert_eq!(static_bool("o"), None);
    assert_eq!(static_bool("$x"), None);
}

#[test]
fn lowering_is_dialect_aware_via_expand_syntax() {
    // Lowering re-segments each
    // body under the document dialect.  For `if {*}$cond { puts hi }`,
    // on 8.5+ the `{*}` expands the condition, so the structured `if`
    // trips the expand-barrier (an opaque `Statement::Barrier` — the
    // structure can't be reasoned about); on 8.4 `{*}` is a literal
    // word, so the `if` lowers normally (no barrier).
    let reg = reg();
    let src = "set cond [gets stdin]\nif {*}$cond { puts hi }";
    let m90 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
    let m84 = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::for_dialect("tcl8.4"));
    let is_barrier = |m: &Module| {
        matches!(
            m.top_level.statements.last(),
            Some(Statement::Barrier { .. })
        )
    };
    assert!(
        is_barrier(&m90),
        "9.0 expands `{{*}}` → structured `if` becomes a barrier: {:?}",
        m90.top_level.statements,
    );
    assert!(
        !is_barrier(&m84),
        "8.4 treats `{{*}}` as literal → `if` lowers without a barrier: {:?}",
        m84.top_level.statements,
    );
}

/// Every structured form `try_dispatch_structured_hook` covers must
/// barrier on `{*}` expansion.  A form that instead commits to its
/// *un-expanded* argv fabricates nonsense IR — `lmap i {*}$spec {…}`
/// becomes an `IRForeach` with one iterator over the literal word
/// `${spec}`, making W210 claim a loop variable is read before it is set.
#[test]
fn every_argv_shaped_hook_barriers_on_expansion() {
    let reg = reg();
    for src in [
        // Forms whose argv shape is fixed by the hook.
        "foreach i {*}$spec {puts $i}",
        "if {*}$cond {puts hi}",
        "switch {*}$spec {a {puts hi}}",
        "while {*}$spec {puts hi}",
        "for {*}$spec {puts hi}",
        "proc {*}$spec {puts hi}",
        "namespace eval {*}$spec {puts hi}",
        // Forms it missed.
        "lmap i {*}$spec {puts $i}",
        "dict for {k v} {*}$rest {puts $k}",
        "array for {k v} {*}$rest {puts $k}",
        "foreachLine ln {*}$rest {puts $ln}",
        "catch {puts hi} {*}$rest",
        "try {puts hi} {*}$rest",
        "eval {*}$rest",
        "uplevel {*}$rest",
        "apply {*}$rest",
    ] {
        // The expanded words hold run-time data: an unset variable would
        // raise before the command ran.
        let src = format!("set spec [gets stdin]\nset cond $spec\nset rest $spec\n{src}");
        let m = lower_to_ir(&src, &reg);
        assert!(
            matches!(
                m.top_level.statements.last(),
                Some(Statement::Barrier { .. })
            ),
            "`{src}` must barrier on `{{*}}` expansion: {:?}",
            m.top_level.statements,
        );
    }
}

/// The barrier is an *expansion* gate, not a blanket one: the same
/// structured forms without `{*}` keep their specialised lowering, and a
/// non-structured hook (`set`) never barriers on expansion at all — its
/// own `has_expansion` check declines and the call reaches
/// `lower_default`.
#[test]
fn expansion_barrier_leaves_unexpanded_and_non_structured_forms_alone() {
    let reg = reg();
    for src in [
        "foreach i $spec {puts $i}",
        "lmap i $spec {puts $i}",
        "catch {puts hi} err",
        "set {*}$rest",
        "puts {*}$rest",
    ] {
        let m = lower_to_ir(src, &reg);
        assert!(
            !matches!(
                m.top_level.statements.first(),
                Some(Statement::Barrier { .. })
            ),
            "`{src}` must not raise the expansion barrier: {:?}",
            m.top_level.statements,
        );
    }
}

/// The barrier reason names the command, which the compiler-explorer
/// `--show ir` view quotes.
#[test]
fn expansion_barrier_reason_names_the_command() {
    let m = lower_to_ir("set spec [gets stdin]\nlmap i {*}$spec {puts $i}", &reg());
    let Some(Statement::Barrier { reason, .. }) = m.top_level.statements.last() else {
        panic!("expected a barrier: {:?}", m.top_level.statements);
    };
    assert_eq!(reason, "lmap with argument expansion");
}

#[test]
fn empty_source() {
    let m = lower_to_ir("", &reg());
    assert_eq!(m.top_level.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn simple_set() {
    let m = lower_to_ir("set x 1", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::AssignConst { name, value, .. } if name == "x" && value == "1"
    ));
}

#[test]
fn set_with_variable_value() {
    let m = lower_to_ir("set x [gets stdin]\nset y $x", &reg());
    assert_eq!(m.top_level.statements.len(), 2);
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::AssignValue { name, .. } if name == "y"
    ));
}

#[test]
fn incr_command() {
    let m = lower_to_ir("incr i", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Incr { name, .. } if name == "i"
    ));
}

// The `extract_oo_methods_pass` post-pass
// lifts TclOO method bodies into `module.methods`.

#[test]
fn extracts_oo_class_methods() {
    let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   constructor {} { set n 0 }\n\
                   \x20   method bump {} { incr n }\n\
                   \x20   method get {} { return $n }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let mut keys: Vec<String> = m
        .methods
        .values()
        .map(crate::ir::MethodDef::reporting_label)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "::Counter::<constructor>".to_string(),
            "::Counter::bump".to_string(),
            "::Counter::get".to_string(),
        ],
        "methods: {keys:?}",
    );
    let get = fixture_method(&m.methods, "::Counter::get").expect("unique retained fixture method");
    assert_eq!(get.method_name, "get");
    assert_eq!(get.kind, MethodKind::Method);
    assert_eq!(get.class_name, "::Counter");
    assert_eq!(
        get.execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected,
        "TclOO resolves relative commands in the runtime receiver namespace"
    );
    // `variable n` in the class body is an in-scope instance var
    // for every method (auto-linked).
    assert!(
        get.instance_vars.contains("n"),
        "get ivars: {:?}",
        get.instance_vars
    );
    // The method body is lowered for analysis (the `incr n` /
    // `return $n` statements are present, not an empty barrier).
    let bump =
        fixture_method(&m.methods, "::Counter::bump").expect("unique retained fixture method");
    assert!(
        !bump.body.statements.is_empty(),
        "bump body should be lowered"
    );
    let ctor = fixture_method(&m.methods, "::Counter::<constructor>")
        .expect("unique retained fixture method");
    assert_eq!(ctor.kind, MethodKind::Constructor);
    assert_eq!(m.redefined_methods.len(), 0);
    assert!(
        !m.oo_evidence.unretained_executable_roots,
        "every executable body in a fully static class was retained"
    );
}

#[test]
fn lowers_oo_configurable_class_body() {
    // `oo::configurable create` (and `oo::abstract` / `oo::singleton`)
    // share the `METACLASS create NAME { body }` shape, so their bodies
    // must be lowered and their methods lifted like `oo::class`'s — not left
    // as an unanalysed barrier, which would skip every method body of a
    // class defined with `oo::configurable`.
    let src = "oo::configurable create Pin {\n\
                   \x20   property node\n\
                   \x20   method describe {} { return [my configure -node] }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Pin::describe"),
        "oo::configurable method must be lifted; methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !fixture_method(&m.methods, "::Pin::describe")
            .expect("unique retained fixture method")
            .body
            .statements
            .is_empty(),
        "the method body should be lowered, not barriered"
    );
}

#[test]
fn extracts_oo_define_methods_and_namespaced_class() {
    // `oo::define` adds methods to an existing class, and a class
    // created inside `namespace eval` qualifies under that ns.
    let src = "namespace eval ::app {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method draw {} { return ok }\n\
                   \x20   }\n\
                   }\n\
                   oo::define ::app::Widget {\n\
                   \x20   method hide {} { return done }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::app::Widget::draw"),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::app::Widget::hide"),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
}

// Extraction is driven by the registry definer grammars, so
// oo::objdefine, snit, and itcl bodies produce method units too.

// TP: an `oo::objdefine $obj { … }` body produces per-object method
// units, so the body is visible to analysis.
#[test]
fn objdefine_bodies_produce_per_object_method_units() {
    let src = "oo::class create C {}\n\
                   set k [C new]\n\
                   oo::objdefine $k {\n\
                   \x20   variable z\n\
                   \x20   method probe {} { return $z }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let unit = m
        .methods
        .unique_display("::@objdefine@::k::probe")
        .map_or_else(
            || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
            |(_, method)| method,
        );
    assert_eq!(unit.method_name, "probe");
    assert!(!unit.body.statements.is_empty(), "body must be lowered");
    // The block's `variable z` is per-object state in scope for the
    // per-object method.
    assert!(
        unit.instance_vars.contains("z"),
        "ivars: {:?}",
        unit.instance_vars
    );
}

// TN: objdefine `variable` declarations are
// per-object — they must NOT pollute the real class's cross-block
// instance-variable union.
#[test]
fn objdefine_variables_stay_out_of_the_class_union() {
    let src = "oo::class create C {\n\
                   \x20   method m {} { info exists z }\n\
                   }\n\
                   set k [C new]\n\
                   oo::objdefine $k { variable z }\n";
    let m = lower_to_ir(src, &reg());
    let class_m = fixture_method(&m.methods, "::C::m").expect("unique retained fixture method");
    assert!(
        !class_m.instance_vars.contains("z"),
        "per-object `z` leaked into the class union: {:?}",
        class_m.instance_vars
    );
}

// TP (item 2): snit method / typemethod / constructor bodies become
// method units, with the grammar's implicit member-body variables
// (self / selfns / type / options) and declared type variables in scope.
#[test]
fn snit_type_bodies_produce_method_units() {
    let src = "snit::type Dog {\n\
                   \x20   variable name\n\
                   \x20   constructor {args} { set name fido }\n\
                   \x20   method bark {} { return \"$name barks\" }\n\
                   \x20   typemethod count {} { return 0 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let bark = m.methods.unique_display("::Dog::bark").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert_eq!(bark.kind, MethodKind::Method);
    assert_eq!(
        bark.execution_namespace,
        crate::ir::ExecutionNamespace::exact("::Dog")
    );
    assert_ne!(bark.body.statements, [] as [crate::ir::Statement; 0]);
    for implicit in ["self", "selfns", "type", "options", "name"] {
        assert!(
            bark.instance_vars.contains(implicit),
            "missing {implicit}: {:?}",
            bark.instance_vars
        );
    }
    assert_eq!(
        fixture_method(&m.methods, "::Dog::count")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::ClassMethod,
        "typemethod dispatches on the type command"
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Dog::<constructor>")
    );
}

// TP: the widget grammar's extra implicit vars (win / hull) reach
// widget method units — registry data, not a name-suffix check.
#[test]
fn snit_widget_bodies_see_win_and_hull() {
    let src = "snit::widget MyBar {\n\
                   \x20   method redraw {} { return $win }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let redraw = m.methods.unique_display("::MyBar::redraw").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert!(redraw.instance_vars.contains("win"));
    assert!(redraw.instance_vars.contains("hull"));
}

// TP (item 2): itcl method bodies — including the access-modifier
// wrapped spellings — become method units with `this`, instance
// variables, and commons in scope.
#[test]
fn itcl_class_bodies_produce_method_units() {
    let src = "itcl::class Toaster {\n\
                   \x20   variable crumbs 0\n\
                   \x20   common heat 3\n\
                   \x20   public method toast {n} { incr crumbs $n }\n\
                   \x20   method clean {} { set crumbs 0 }\n\
                   \x20   constructor {} { set crumbs 0 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let toast = m.methods.unique_display("::Toaster::toast").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert_eq!(toast.params, vec!["n".to_string()]);
    assert_eq!(
        toast.execution_namespace,
        crate::ir::ExecutionNamespace::exact("::Toaster")
    );
    for name in ["this", "crumbs", "heat"] {
        assert!(
            toast.instance_vars.contains(name),
            "missing {name}: {:?}",
            toast.instance_vars
        );
    }
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Toaster::clean")
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Toaster::<constructor>")
    );
}

#[test]
fn definition_member_words_use_positioned_source_and_retained_grammar() {
    // Software source projection only: no runtime member or frame grant.
    use crate::registry_invocation::EffectiveInvocationWord as Word;
    let source = "method {a\\\n  b} \"$name\" {*}$args";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    for (config, expected) in [
        (config, "a b"),
        (
            tcl_lexer::LexerConfig {
                brace_backslash_newline: tcl_dialect::BraceBackslashNewline::Literal,
                ..config
            },
            "a\\\n  b",
        ),
    ] {
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .expect("one original member invocation");
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        assert_eq!(
            statement_words(&tokens, config),
            vec![
                Word::Literal("method".to_owned()),
                Word::Literal(expected.to_owned()),
                Word::Dynamic,
                Word::Expanded,
            ]
        );
    }
}

// TP: TclOO wrapper members — `self { method … }` lifts a
// classmethod-kind unit, `private { method … }` an instance one; the
// prefix spellings match.
#[test]
fn tcloo_wrapper_members_produce_units() {
    let src = "oo::class create W {\n\
                   \x20   self { method make {} { return [my new] } }\n\
                   \x20   private { method hidden {} { return 1 } }\n\
                   \x20   self method direct {} { return 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert_eq!(
        fixture_method(&m.methods, "::W::make").map_or_else(
            || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
            |d| d.kind
        ),
        MethodKind::ClassMethod
    );
    assert_eq!(
        fixture_method(&m.methods, "::W::direct")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::ClassMethod
    );
    assert_eq!(
        fixture_method(&m.methods, "::W::hidden")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::Method
    );
    assert!(!m.oo_evidence.unretained_executable_roots);
}

// TN: an `initialise` body is a *definition script* evaluated in the
// class object's namespace, not a method frame — no unit.
#[test]
fn tcloo_initialise_body_is_not_a_method_unit() {
    let src = "oo::class create I {\n\
                   \x20   initialise { variable cache {} }\n\
                   \x20   method read {} { return 1 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        !m.methods
            .values()
            .any(|m| m.reporting_label().contains("initialise")),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::I::read")
    );
    assert!(
        m.oo_evidence.unretained_executable_roots,
        "the unrepresented initialise script must remain explicit evidence"
    );
}

// TN: `self constructor` / `self destructor` name no member (tclsh
// 8.6.18, 9.0.4 and 9.1: `invalid command name "constructor"`), so
// neither lifts a unit —
// while the instance-side spellings beside them do.
#[test]
fn tcloo_self_constructor_and_destructor_lift_no_unit() {
    let src = "oo::class create S {\n\
                   \x20   self constructor {} { set a 1 }\n\
                   \x20   self destructor { set b 2 }\n\
                   \x20   method m {} { return 3 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .all(|d| !matches!(d.kind, MethodKind::Constructor | MethodKind::Destructor)),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::S::m")
    );
    let src = "oo::class create T {\n\
                   \x20   constructor {} { set a 1 }\n\
                   \x20   destructor { set b 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert_eq!(
        fixture_method(&m.methods, "::T::<constructor>")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::Constructor
    );
    assert!(
        m.methods.values().any(|d| d.kind == MethodKind::Destructor),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
}

// TN: a dynamic objdefine receiver that is not a simple variable
// reference (a command substitution) stays un-extracted.
#[test]
fn objdefine_command_substitution_receiver_abstains() {
    let src = "oo::class create C {}\n\
                   oo::objdefine [C new] { method m {} { return 1 } }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        !m.methods
            .values()
            .any(|m| m.reporting_label().starts_with("::@objdefine@")),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(m.oo_evidence.unretained_executable_roots);
}

#[test]
fn colliding_method_reports_retain_distinct_bodies_and_constructor_purposes() {
    let source = "namespace eval A {}\noo::class create A::B {method c {} {return LEFT}}\noo::class create A {method B::c {} {return RIGHT}; method <constructor> {} {return NAMED}; constructor {} {set local ACTUAL}; export B::c <constructor>}\n";
    let module = lower_to_ir(source, &reg());
    assert_eq!(module.methods.len(), 4);
    assert_eq!(
        module
            .methods
            .keys()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert!(module.methods.unique_display("::A::B::c").is_none());
    assert!(
        module
            .methods
            .unique_display("::A::<constructor>")
            .is_none()
    );
    assert_eq!(
        module
            .methods
            .values()
            .filter(|method| method.kind == MethodKind::Constructor)
            .count(),
        1
    );
    assert_eq!(
        module
            .methods
            .values()
            .filter(
                |method| method.kind == MethodKind::Method && method.method_name == "<constructor>"
            )
            .count(),
        1
    );
    assert!(module.redefined_methods.is_empty());
    for id in module.methods.keys() {
        assert_eq!(module.method_definition_bodies(*id).count(), 1);
    }
}

#[test]
fn redefined_oo_method_retains_the_replacement_body() {
    // A method redefined by a later `oo::define` keeps the first
    // body in `methods`, and RETAINS the replacement in
    // `redefined_methods` so analysis consumers can
    // scan every body a dispatch may run instead of abstaining.
    let src = "oo::class create C {\n\
                   \x20   method m {} { return 1 }\n\
                   }\n\
                   oo::define C {\n\
                   \x20   method m {} { return 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
    let (primary, replacements) = m
        .redefined_methods
        .iter()
        .next()
        .unwrap_or_else(|| panic!("redefined: {:?}", m.redefined_methods));
    assert_eq!(m.methods.get(primary).unwrap().method_name, "m");
    assert_eq!(replacements.len(), 1, "one replacement body retained");
    let retained = m.methods.get(&replacements[0]).unwrap();
    assert_eq!(retained.class_name, "::C");
    assert!(
        !retained.body.statements.is_empty(),
        "the replacement body is lowered, not discarded"
    );
    assert_eq!(m.oo_unanalysed_classes.len(), 0);
    assert!(!m.oo_evidence.dynamic_target);
    assert!(!m.oo_evidence.unretained_executable_roots);
}

#[test]
fn unreadable_oo_members_flag_the_class_or_module() {
    // A dynamic member NAME may redefine any method of the class —
    // the class is flagged unanalysable.
    let m = lower_to_ir(
        "oo::class create C {\n    method m {} { return 1 }\n    method $n {} { return 2 }\n}\n",
        &reg(),
    );
    assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
    assert!(m.oo_evidence.unretained_executable_roots);
    // A dynamic member BODY is a method whose code no scan can read.
    let m = lower_to_ir("oo::class create C {\n    method m {} $body\n}\n", &reg());
    assert!(m.oo_unanalysed_classes.contains("::C"));
    assert!(m.oo_evidence.unretained_executable_roots);
    // A dynamic CLASS word may touch any class — module-wide flag.
    let m = lower_to_ir("oo::define $cls { method m {} { return 1 } }\n", &reg());
    assert!(m.oo_evidence.dynamic_target);
    assert!(m.oo_evidence.unretained_executable_roots);
    // A fully-static module sets neither.
    let m = lower_to_ir("oo::class create C { method m {} { return 1 } }\n", &reg());
    assert_eq!(m.oo_unanalysed_classes.len(), 0);
    assert!(!m.oo_evidence.dynamic_target);
    assert!(!m.oo_evidence.unretained_executable_roots);
}

#[test]
fn every_unextracted_oo_body_shape_leaves_module_evidence() {
    for source in [
        // The outer definition script is computed, so the definer post-pass
        // cannot segment it.
        "oo::class create C [list method mutate {} { return 1 }]",
        // Inline oo::define is body-bearing but is not a braced definition
        // block accepted by classify_definer_call.
        "oo::class create C {}; oo::define C method mutate {} { return 1 }",
        // Accessor and configuration scripts are deliberately not MethodDef
        // frames, but remain runtime-callable executable bodies.
        "oo::configurable create C { property p -get { return 1 } }",
        "itcl::class C { variable value 0 { return 1 } }",
        // A definition created when an already-extracted method runs is not
        // visited by this source-root post-pass.
        "oo::class create Outer { method install {} { oo::class create Inner { method mutate {} { return 1 } } } }",
        // Definition scripts may invoke ordinary commands; such execution
        // is not represented by a lifted method body.
        "oo::class create C { ::rename ::set ::saved_set }",
    ] {
        let module = lower_to_ir(source, &reg());
        assert!(
            module.oo_evidence.unretained_executable_roots,
            "unretained executable definition body was not recorded: {source}"
        );
    }
}

#[test]
fn oo_method_nested_proc_not_registered_globally() {
    // A `proc` defined inside a method body is created at
    // method-call time, not script load — it must not leak into
    // `module.procedures` (codegen safety).
    let src = "oo::class create C {\n\
                   \x20   method m {} { proc helper {} { return 1 }; return 2 }\n\
                   }\n\
                   proc toplevel {} { return 3 }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::toplevel").is_some(),
        "procs: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_none(),
        "nested method proc must not be registered: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    // The method body still lowered the `proc helper ...` call.
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
}

#[test]
fn apply_body_proc_registers_in_lambda_namespace_not_callers() {
    // `apply`'s body runs in the namespace named by the lambda — or the
    // *global* namespace when none is given — NOT the caller's namespace.
    // So a nested `proc` registers as `::helper`, never `::foo::helper`
    // (Tcl `apply` manual).
    let src = "namespace eval foo {\n\
                   \x20   apply {{} { proc helper {} { return 1 } }}\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_some(),
        "apply body proc must register in the global namespace: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::foo::helper").is_none(),
        "apply body proc must NOT inherit the caller namespace: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn apply_body_proc_uses_explicit_lambda_namespace() {
    // A lambda whose element 2 pins a namespace runs its body there.
    let src = "apply {{} { proc helper {} { return 1 } } ::bar}\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::bar::helper").is_some(),
        "explicit lambda namespace must be honoured: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

/// TP — a relative lambda namespace element homes against the GLOBAL
/// namespace even from inside a qualified-name proc (`doc/apply.n`:
/// "interpreted relative to the global namespace even if its name does
/// not start with ::"; tclsh 9.0.4 probe `a2b_apply.tcl`). Mirrors the
/// analyser's `handle_apply_command` so IR and analysis agree.
#[test]
fn apply_relative_lambda_namespace_homes_globally_not_to_the_caller() {
    let src = "proc ::caller::p {} { apply {{} { proc helper {} { return 1 } } sub} }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::sub::helper").is_some(),
        "relative lambda ns must resolve against ::: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::caller::sub::helper").is_none(),
        "relative lambda ns must NOT pin against the caller: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn no_oo_methods_for_plain_script() {
    let m = lower_to_ir("proc greet {} { puts hi }", &reg());
    assert_eq!(m.methods.len(), 0);
    assert_eq!(m.redefined_methods.len(), 0);
}

#[test]
fn proc_definition() {
    let m = lower_to_ir("proc greet {name} {puts $name}", &reg());
    // proc emits a Statement::Call + registers a procedure.
    assert!(fixture_id_if_present(&m.procedures, "::greet").is_some());
    let p = m
        .procedures
        .get(&fixture_id(&m.procedures, "::greet"))
        .expect("authored fixture body");
    assert_eq!(p.params, vec!["name"]);
}

#[test]
fn if_statement() {
    let m = lower_to_ir("if {1} {set x 1}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(&m.top_level.statements[0], Statement::If { .. }));
}

#[test]
fn logical_structure_does_not_grant_native_compilation() {
    let registry = reg();
    let source = "if {1} {set x 1}";
    let logical = lower_to_ir(source, &registry);
    assert!(matches!(
        logical.top_level.statements.first(),
        Some(Statement::If { .. })
    ));
    let mut native = Lowerer::new(&registry).for_bytecode_backend();
    native.source_entry_origin = SourceEntryOrigin::Driver;
    native.native_compilation.mode =
        tcl_registry::native_compilation::NativeCompilationMode::Direct;
    let emitted = lower_to_ir_with(native, source);
    let Some(Statement::Call {
        tokens: Some(tokens),
        ..
    }) = emitted.top_level.statements.first()
    else {
        panic!(
            "direct source entry has no admitted native compiler: {:?}",
            emitted.top_level
        );
    };
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(&registry, None, tokens)
            .is_none()
    );
}

#[test]
fn for_loop() {
    let m = lower_to_ir("for {set i 0} {$i < 10} {incr i} {puts $i}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(&m.top_level.statements[0], Statement::For { .. }));
}

#[test]
fn while_loop() {
    let m = lower_to_ir("while {1} {puts loop}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::While { .. }
    ));
}

#[test]
fn foreach_loop() {
    let m = lower_to_ir("foreach x {a b c} {puts $x}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Foreach { .. }
    ));
}

#[test]
fn catch_statement() {
    let m = lower_to_ir("catch {error oops} result", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Catch { .. }
    ));
}

#[test]
fn generic_command() {
    let m = lower_to_ir("puts hello", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Call { command, .. } if command == "puts"
    ));
}

#[test]
fn multiple_commands() {
    let m = lower_to_ir("set x 1\nset y 2\nputs $x", &reg());
    assert_eq!(m.top_level.statements.len(), 3);
}

#[test]
fn return_statement() {
    let m = lower_to_ir("return 42", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Return { value: Some(v), .. } if v == "42"
    ));
}

#[test]
fn returned_module_snapshots_retain_their_actual_source_owner() {
    use std::sync::Arc;

    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let context = Arc::new(generation.with_command_store(Arc::clone(generation.commands())));
    let registry = generation.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let source = "throw ERROR payload";
    for target in 0..3 {
        let mut lowerer = Lowerer::with_config(registry, config)
            .with_dialect(registry.profile())
            .with_context_registry(Arc::clone(&context));
        let module = match target {
            0 => lowerer.lower(source),
            1 => lowerer.lower_procedure_target(source, "::N"),
            _ => lowerer.lower_script_target(source, "::N"),
        };
        let owner = module
            .retained_source_bindings
            .as_ref()
            .expect("actual completed source");
        assert_eq!(module.source.bytes(), source.as_bytes());
        assert!(
            owner.positioned_for_module(module, registry).is_some(),
            "{target}"
        );
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::document("throw ERROR changed");
        assert!(owner.positioned_for_module(&changed, registry).is_none());
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(owner.positioned_for_module(&changed, registry).is_none());
    }
}

#[test]
fn lower_to_ir_public_api() {
    let r = reg();
    let m = lower_to_ir("set x 1\nproc foo {} {return 1}", &r);
    assert_eq!(m.top_level.statements.len(), 2);
    assert!(fixture_id_if_present(&m.procedures, "::foo").is_some());
}

// A proc body homes nested definitions to the proc's *defining*
// namespace, not the lexical namespace of the `proc` call.

#[test]
fn analyser_settings_preserve_factory_through_original_unit_rebuild() {
    let source = "set value VALUE";
    let mut analyser = crate::analyser::Analyser::new();
    let settings = analyser.source_compilation_settings();
    let analysis = analyser.analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let options = crate::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: analyser.declared_commands.as_ref(),
    };
    let fresh = |unit: &crate::compilation_unit::CompilationUnit| {
        unit.ir_module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(&unit.ir_module, registry)
            .is_some()
    };
    let unit =
        crate::compilation_unit::CompilationUnit::build_with_analysis_input_and_source_settings(
            source, options, None, input, &settings,
        );
    assert!(fresh(&unit));
    assert_eq!(
        unit.ir_module
            .source_entry
            .analyser_source_settings
            .as_ref(),
        Some(&settings)
    );
    let rebuilt = analysis
        .original_optimiser_source_unit(source, registry, None)
        .unwrap();
    assert!(fresh(&rebuilt));
    assert_eq!(
        unit.original_optimiser_source_origin(),
        rebuilt.original_optimiser_source_origin()
    );
    let entry = unit.ir_module.source_entry.clone();
    let explicit =
        crate::compilation_unit::CompilationUnit::build_with_analysis_input_and_source_settings(
            source,
            options,
            Some(&entry),
            input,
            &settings,
        );
    assert_eq!(explicit.ir_module.source_entry, entry);
    assert!(!fresh(&explicit));
}

#[test]
fn explicit_equal_options_revoke_supplied_source_issuance() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(context.commands()))),
        config,
    );
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(Some(profile))
        .with_resolved_analysis_input(input);
    let source = "set value VALUE";
    let original = lowerer.lower(source);
    assert!(
        original
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(original, registry)
            .is_some()
    );
    let entry = original.source_entry.clone();
    lowerer.set_source_analysis_options(entry.options());
    assert!(lowerer.supplied_source.is_none());
    let explicit = lowerer.lower(source);
    assert_eq!(explicit.source_entry, entry);
    assert!(
        explicit
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(explicit, registry)
            .is_none()
    );
}

#[test]
fn configured_source_ingress_retains_one_input_for_entry_and_module() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let source = "proc f {p} {return $p}";
    let module = lower_to_ir_with_dialect(source, registry, config, Some(profile));
    let input = module
        .source_metadata_input
        .as_ref()
        .expect("actual configured source input");
    assert_eq!(
        module.source_entry.metadata_context.source_analysis_input(),
        Some(input)
    );
    assert!(module.source_entry.logical_source_input.is_none());
    assert_eq!(
        module
            .source_entry
            .options()
            .execution_name_policy()
            .unwrap()
            .native_recipe()
            .unwrap()
            .authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert!(module.source_entry.native_entry.is_none());
    assert!(
        module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .matches_module(&module, registry)
    );
    let procedures = module.procedures.iter().collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    assert!(module.procedure_ids.owns(*procedures[0].0));
    assert_eq!(procedures[0].1.qualified_name, "::f");
}

#[test]
fn original_name_operand_publishes_a_body_without_borrowing_the_head_lookup() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let source = r#"proc f {p} {set b [set {$p}]; set a 1; set a 2; return "$a$b"}"#;
    let module = lower_to_ir_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(profile),
    );
    let procedures = module.procedures.iter().collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    let (id, procedure) = procedures[0];
    assert!(module.procedure_ids.owns(*id));
    assert_eq!(procedure.qualified_name, "::f");
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(Statement::tokens)
        .expect("the actual original declaration command");
    let binding = tokens
        .source_binding
        .as_ref()
        .expect("original source binding");
    let input = binding
        .original_written_name_input(tokens, 1)
        .expect("actual original name operand");
    assert!(
        binding.original_command_lookup(tokens, &input).is_none(),
        "an operand cannot replace the original command-head owner"
    );
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(
        procedure.declaration_site.as_ref(),
        binding.invocation_site()
    );
    let head = binding
        .original_written_name_input(tokens, 0)
        .expect("actual original declaration head");
    assert_eq!(head.bytes(), b"proc");
    assert_eq!(input.bytes(), b"f");
    assert_ne!(
        head.original_word_key().unwrap().original_word(),
        input.original_word_key().unwrap().original_word()
    );
    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) = &input else {
        panic!("original literal name word");
    };
    assert_eq!(
        key.policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    if let Some(scope) = binding.original_operand_naming_scope(&input) {
        assert_eq!(
            procedure.publication.as_ref(),
            crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(&scope, key)
                .as_ref()
        );
    } else {
        assert!(procedure.publication.is_none());
    }
}

#[test]
fn proc_body_namespace_peels_the_names_own_qualifier() {
    // Unit-level: the helper is the inverse of `qualify_proc_name`.
    assert_eq!(proc_body_namespace("::a::outer", "::"), "::a");
    assert_eq!(proc_body_namespace("::b::c::o2", "::"), "::b::c");
    assert_eq!(proc_body_namespace("::plain", "::ignored"), "::");
    // A bare (unrooted) key can't come out of `qualify_proc_name`, but the
    // fallback must not manufacture an empty namespace if it ever did.
    assert_eq!(proc_body_namespace("plain", "::lex"), "::lex");
}

#[test]
fn nested_proc_homes_to_the_outer_procs_defining_namespace() {
    // TP. On tclsh 9.0.4 and 8.6.16, identically:
    //   namespace eval ::a {}
    //   proc a::outer {} { proc helper {x} {return $x} }
    //   a::outer ; info commands ::a::helper   -> ::a::helper
    //              info commands ::helper      -> {}
    let m = lower_to_ir(
        "namespace eval ::a {}\nproc a::outer {} { proc helper {x} { return $x } }\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::a::helper").is_some(),
        "nested proc must home to ::a, got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_none(),
        "nested proc must NOT home lexically to ::",
    );
}

#[test]
fn nested_proc_homing_composes_through_three_levels() {
    // TP. On tclsh, `proc g::o7 {} { proc o7b {} { proc h7 {u} … } }` →
    // running `g::o7` then `::g::o7b` leaves `::g::h7` (never `::h7`).
    let m = lower_to_ir(
        "namespace eval ::g {}\nproc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }\n",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::g::o7b").is_some());
    assert!(
        fixture_id_if_present(&m.procedures, "::g::h7").is_some(),
        "third level must stay in ::g, got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::h7").is_none());
}

#[test]
fn nested_proc_name_qualifier_beats_the_lexical_namespace_eval() {
    // TP. On tclsh, inside `namespace eval ::d`, `proc ::e::o3` still homes
    // its body to `::e` — `::e::h3` exists, `::d::h3` and `::h3` do not.
    let m = lower_to_ir(
        "namespace eval ::e {}\nnamespace eval ::d {\n    proc ::e::o3 {} { proc h3 {z} { return $z } }\n}\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::e::h3").is_some(),
        "got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::d::h3").is_none());
    assert!(fixture_id_if_present(&m.procedures, "::h3").is_none());
}

#[test]
fn unqualified_outer_proc_leaves_nested_homing_unchanged() {
    // TN. The common shape: lexical and defining namespace agree, so
    // homing changes nothing. Both at top level…
    let m = lower_to_ir("proc outer {} { proc inner {} { return 1 } }\n", &reg());
    assert!(fixture_id_if_present(&m.procedures, "::inner").is_some());
    // …and inside a `namespace eval`, where both are `::ns`.
    let m = lower_to_ir(
        "namespace eval ::ns {\n    proc outer {} { proc inner {} { return 1 } }\n}\n",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::ns::inner").is_some(),
        "got {:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn absolutely_named_nested_proc_is_unaffected() {
    // TN. On tclsh, `proc a::outer4 {} { proc ::abs {q} … }` creates `::abs`
    // — an absolute inner name ignores the enclosing frame entirely.
    let m = lower_to_ir(
        "namespace eval ::a {}\nproc a::outer4 {} { proc ::abs {q} { return $q } }\n",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::abs").is_some());
    assert!(fixture_id_if_present(&m.procedures, "::a::abs").is_none());
}

#[test]
fn original_formal_name_headers_use_selected_grammar_and_keep_activation_separate() {
    // naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    let tcl = Some(tcl_dialect::ParameterGrammar::Tcl);
    let jim = Some(tcl_dialect::ParameterGrammar::Jim);
    assert_eq!(parse_formal_param_names("{a::b}", FOLD_RULES, tcl), None);
    assert_eq!(
        parse_formal_param_names("{a::b}", LITERAL_RULES, jim),
        Some(vec!["a::b".to_owned()])
    );
    assert_eq!(parse_formal_param_names("{a(1)}", FOLD_RULES, tcl), None);
    assert_eq!(
        parse_formal_param_names("{a(1)}", LITERAL_RULES, jim),
        Some(vec!["a(1)".to_owned()])
    );
    // Name-only headers retain original link spelling. They cannot claim
    // the ordinary local storage name or manufacture a caller-link frame.
    assert_eq!(
        parse_formal_param_names("&a {&b default}", LITERAL_RULES, jim),
        Some(vec!["&a".to_owned(), "&b".to_owned()])
    );
    assert_eq!(
        parse_formal_param_names("args args", LITERAL_RULES, jim),
        None
    );
    assert_eq!(
        parse_formal_param_names("args args", FOLD_RULES, tcl),
        Some(vec!["args".to_owned(), "args".to_owned()])
    );
    assert_eq!(parse_formal_param_names("a b", FOLD_RULES, None), None);
}

#[test]
fn formal_param_names_take_the_specifier_name_only() {
    assert_eq!(
        parse_formal_param_names(
            "a b c",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"],
    );
    assert_eq!(
        parse_formal_param_names(
            "{x default} y",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["x", "y"]
    );
    assert_eq!(
        parse_formal_param_names("", FOLD_RULES, Some(tcl_dialect::ParameterGrammar::Tcl)).unwrap(),
        [] as [std::string::String; 0]
    );
    // A wrapped parameter list collapses its continuation before splitting.
    assert_eq!(
        parse_formal_param_names(
            "a b\\\n    c",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        )
        .unwrap(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn formal_param_names_reject_what_tcl_rejects() {
    // Tcl refuses to create these procedures, so lowering declines too.
    assert_eq!(
        parse_formal_param_names(
            "{a b c}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names(
            "{a::b}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names(
            "{a(1)}",
            FOLD_RULES,
            Some(tcl_dialect::ParameterGrammar::Tcl)
        ),
        None
    );
    assert_eq!(
        parse_formal_param_names("{a", FOLD_RULES, Some(tcl_dialect::ParameterGrammar::Tcl)),
        None
    );
}

#[test]
fn var_list_names_keep_grouped_elements_whole() {
    assert_eq!(
        parse_var_list_names("a b", FOLD_RULES).unwrap(),
        vec!["a", "b"]
    );
    // tclsh 9.0: both spellings bind the one variable named `a b`.
    assert_eq!(
        parse_var_list_names("{a b}", FOLD_RULES).unwrap(),
        vec!["a b"]
    );
    assert_eq!(
        parse_var_list_names("a\\ b", FOLD_RULES).unwrap(),
        vec!["a b"]
    );
    assert_eq!(
        parse_var_list_names("", FOLD_RULES).unwrap(),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        parse_var_list_names("a b\\\n    c", FOLD_RULES).unwrap(),
        vec!["a", "b", "c"]
    );
    // Unbalanced — no binding can be named, so the caller barriers.
    assert_eq!(parse_var_list_names("{a", FOLD_RULES), None);
}

// A formal-parameter list has two list levels, so a grouped or escaped
// specifier is one parameter, not one name per whitespace run:
// `proc p {a\ b} {}` has arity one.

/// Parameter names recorded for `::p` when `src` is lowered.
fn proc_params(src: &str) -> Vec<String> {
    let m = lower_to_ir(src, &reg());
    fixture_id_if_present(&m.procedures, "::p")
        .and_then(|id| m.procedures.get(&id))
        .unwrap_or_else(|| panic!("::p registered for {src:?}"))
        .params
        .clone()
}

#[test]
fn escaped_space_proc_param_is_one_parameter() {
    // tclsh 9.0: `proc p {a\ b} {}` → `info args p` is `a`, with
    // `info default p a` yielding `b`. Only the name reaches the IR, so
    // the arity is one, not two (`a\` and `b`).
    assert_eq!(proc_params("proc p {a\\ b} {}"), vec!["a"]);
    // The grouped spelling decodes identically.
    assert_eq!(proc_params("proc p {{a b}} {}"), vec!["a"]);
}

#[test]
fn proc_params_keep_defaults_and_args_as_names() {
    // FP-guard: `{x {y 2} args}` is still the three parameters `x`, `y`,
    // and the variadic `args`.
    assert_eq!(
        proc_params("proc p {x {y 2} args} {}"),
        vec!["x", "y", "args"]
    );
    assert_eq!(proc_params("proc p {} {}"), [] as [std::string::String; 0]);
}

#[test]
fn proc_params_tcl_rejects_fall_through_to_barrier() {
    // tclsh: `proc p {{a b c}} {}` → "too many fields in argument
    // specifier". No procedure is created, so lowering registers none and
    // leaves the runtime `proc` to report it.
    for src in [
        "proc p {{a b c}} {}",
        "proc p {a::b} {}",
        "proc p {a(1)} {}",
    ] {
        let m = lower_to_ir(src, &reg());
        assert!(
            fixture_id_if_present(&m.procedures, "::p").is_none(),
            "no ::p for {src:?}: {:?}",
            m.procedures.keys().collect::<Vec<_>>(),
        );
        assert!(
            matches!(&m.top_level.statements[0], Statement::Barrier { reason, .. } if reason == "malformed proc params"),
            "expected a malformed-params barrier for {src:?}, got {:?}",
            m.top_level.statements[0],
        );
    }
}

#[test]
fn oo_method_params_use_the_formal_parameter_grammar() {
    let m = lower_to_ir(
        "oo::class create C {\n    method m {a\\ b {c 1}} { return 1 }\n}\n",
        &reg(),
    );
    let method = fixture_method(&m.methods, "::C::m").unwrap_or_else(|| {
        panic!(
            "::C::m lowered, got {:?}",
            m.methods.keys().collect::<Vec<_>>()
        )
    });
    assert_eq!(method.params, vec!["a", "c"]);
    // A parameter list Tcl rejects makes the whole class unanalysable.
    let m = lower_to_ir(
        "oo::class create C {\n    method m {{a b c}} { return 1 }\n}\n",
        &reg(),
    );
    assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
    assert!(
        !m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
}

#[test]
fn qualify_proc_name_global() {
    assert_eq!(qualify_proc_name("::", "foo"), "::foo");
}

#[test]
fn qualify_proc_name_nested() {
    assert_eq!(qualify_proc_name("::ns", "bar"), "::ns::bar");
}

#[test]
fn qualify_proc_name_already_qualified() {
    assert_eq!(qualify_proc_name("::ns", "::abs"), "::abs");
}

#[test]
fn qualify_proc_name_preserves_colon_named_namespace_keys() {
    assert_eq!(qualify_proc_name(":::", ":"), "::::::");
    assert_eq!(qualify_proc_name("::a", "a:::b"), "::a::a::b");
    assert_eq!(tcl_syntax::naming::key_holder_and_tail("::::::").0, ":::");
}

#[test]
fn parse_uplevel_level_decimal() {
    let relative = |shift| {
        Some(UplevelLevel {
            shift,
            absolute: false,
        })
    };
    assert_eq!(parse_uplevel_level("1"), relative(1));
    assert_eq!(parse_uplevel_level("3"), relative(3));
    assert_eq!(parse_uplevel_level("0"), relative(0));
}

#[test]
fn parse_uplevel_level_hash_form() {
    let absolute = |shift| {
        Some(UplevelLevel {
            shift,
            absolute: true,
        })
    };
    assert_eq!(parse_uplevel_level("#0"), absolute(0));
    assert_eq!(parse_uplevel_level("#3"), absolute(3));
}

#[test]
fn parse_uplevel_level_zero_keeps_absolute_and_relative_apart() {
    // `uplevel #0` runs the body in the global frame; `uplevel 0` runs it
    // in the current frame. Both carry the magnitude 0, so only the
    // `absolute` flag tells them apart — conflating them miscompiled the
    // command-word namespace of every `uplevel 0` body.
    let hash_zero = parse_uplevel_level("#0").expect("#0 parses");
    let bare_zero = parse_uplevel_level("0").expect("0 parses");
    assert_eq!(hash_zero.shift, bare_zero.shift);
    assert!(hash_zero.absolute);
    assert!(!bare_zero.absolute);
    assert_ne!(hash_zero, bare_zero);
}

#[test]
fn parse_uplevel_level_dynamic_returns_none() {
    assert_eq!(parse_uplevel_level("$lvl"), None);
    assert_eq!(parse_uplevel_level("[expr {1+1}]"), None);
    assert_eq!(parse_uplevel_level("foo"), None);
}

#[test]
fn uplevel_static_body_no_level() {
    // A bare `uplevel {body}` means level 1, which raises `bad level "1"`
    // at the global level in every C release, so the form is lowered
    // where it runs: inside a procedure.
    let m = lower_to_ir("proc f {} {uplevel {set x 1}}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert_eq!(proc.body.statements.len(), 1);
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift, body, ..
        } => {
            assert_eq!(*frame_shift, 1);
            assert_eq!(body.statements.len(), 1);
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_level_one() {
    // `uplevel 1` raises `bad level "1"` at the global level.
    let m = lower_to_ir("proc f {} {uplevel 1 {set x 1}}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 1);
            assert!(!*absolute, "`uplevel 1` is the relative form");
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_hash_zero() {
    let m = lower_to_ir("uplevel #0 {set x 1}", &reg());
    match &m.top_level.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 0);
            assert!(*absolute, "`uplevel #0` is the absolute global form");
        }
        other => panic!("expected UpFrame for #0, got {other:?}"),
    }
}

#[test]
fn uplevel_static_body_with_bare_zero_is_relative() {
    // `uplevel 0 {body}` evaluates the body in the *current* frame, not
    // the global one. It shares the magnitude 0 with `#0` and is told
    // apart only by `absolute`.
    let m = lower_to_ir("uplevel 0 {set x 1}", &reg());
    match &m.top_level.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            ..
        } => {
            assert_eq!(*frame_shift, 0);
            assert!(!*absolute, "`uplevel 0` is the relative current-frame form");
        }
        other => panic!("expected UpFrame for bare 0, got {other:?}"),
    }
}

#[test]
fn uplevel_dynamic_body_falls_back_to_default() {
    // ``uplevel 1 $body`` body is a $var, not a brace literal —
    // can't be statically resolved without const-propagation.
    // Falls back to ``lower_default`` (Statement::Call).
    let m = lower_to_ir("uplevel 1 $body", &reg());
    assert!(matches!(
        m.top_level.statements[0],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn uplevel_dynamic_level_falls_back_to_default() {
    // ``uplevel $lvl {body}`` — level is dynamic, can't pick a
    // ``frame_shift``. Falls back to default lowering.
    let m = lower_to_ir("uplevel $lvl {set x 1}", &reg());
    assert!(matches!(
        m.top_level.statements[0],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn const_prop_uplevel_resolves_set_var_body() {
    // ``set body {set x 1}; uplevel 1 $body`` inside a
    // proc — the const-map records ``body`` and the uplevel
    // folds in the literal as an UpFrame.
    let m = lower_to_ir("proc f {} { set body {set x 1}\n uplevel 1 $body }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    // proc body: [AssignConst(body), UpFrame { body: [...] }]
    let last = proc.body.statements.last().expect("body has statements");
    match last {
        Statement::UpFrame { body, .. } => {
            assert!(!body.statements.is_empty(), "expected lowered body");
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn const_prop_eval_resolves_set_var_body() {
    // ``eval $body`` with const-mapped body folds to a
    // Statement::Block.
    let m = lower_to_ir("proc f {} { set body {set x 1}\n eval $body }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let last = proc.body.statements.last().expect("body has statements");
    match last {
        Statement::Block { body, .. } => {
            assert!(!body.statements.is_empty(), "expected lowered block");
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[test]
fn const_prop_eval_brace_body_emits_block() {
    // ``eval {body}`` in a proc context lowers to Block.
    let m = lower_to_ir("proc f {} { eval {set x 1} }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_disabled_at_top_level() {
    // The const-map is gated on ``proc_depth > 0``. At
    // top-level, ``set body {set x 1}; uplevel 1 $body``
    // does NOT relax — the uplevel falls back to default
    // dispatch (Call / Barrier).
    let m = lower_to_ir("set body {set x 1}\nuplevel 1 $body", &reg());
    // statements[0] = AssignConst(body), [1] = uplevel call
    assert!(matches!(
        m.top_level.statements[1],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn const_prop_invalidated_on_reassignment() {
    // ``set body {a}; set body $other; uplevel 1 $body`` — the
    // second ``set`` invalidates the binding (RHS is a $var,
    // not a brace literal), so the uplevel can't fold.
    let m = lower_to_ir(
        "proc f {} { set body {a}\n set body $other\n uplevel 1 $body }",
        &reg(),
    );
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let last = proc.body.statements.last().expect("body");
    assert!(
        !matches!(last, Statement::UpFrame { .. }),
        "expected fallback after re-assignment, got {last:?}",
    );
}

#[test]
fn const_prop_eval_list_literal() {
    // ``eval [list set x 42]`` recognised as static body.
    let m = lower_to_ir("proc f {} { eval [list set x 42] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_eval_list_with_dynamic_arg_rejected() {
    // ``eval [list set x $v]`` — dynamic ``\$v`` rejects the
    // list-literal shape. Falls back to runtime barrier.
    let m = lower_to_ir("proc f {} { eval [list set x $v] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_eval_non_list_command_rejected() {
    // ``eval [foo arg]`` — inner command isn't ``list``;
    // can't synthesise a body. Falls back to runtime barrier.
    let m = lower_to_ir("proc f {} { eval [foo arg] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_does_not_leak_into_nested_proc() {
    // A ``set body {literal}`` in the outer proc must
    // NOT appear to a nested ``proc inner``'s
    // barrier-relaxation gate as a tracked literal.
    let m = lower_to_ir(
        "proc outer {} {\n  set body {set x 1}\n  proc inner {} { uplevel 1 $body }\n}",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::inner")
        .and_then(|id| m.procedures.get(&id))
        .expect("inner registered");
    // The inner uplevel must remain a Call/Barrier — NOT an
    // UpFrame. If the const-map leaked, the inner body would
    // be folded as UpFrame { body: [set x 1], .. }.
    let last = inner.body.statements.last().expect("body");
    assert!(
        !matches!(last, Statement::UpFrame { .. }),
        "outer scope's const-map must not leak into nested proc, got {last:?}",
    );
}

#[test]
fn ns_import_recorded_with_context_namespace() {
    // ``namespace import ::tcltest::*`` at top-level
    // records (``::``, ``::tcltest::*``).
    let m = lower_to_ir("namespace import ::tcltest::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_string(), "::tcltest::*".to_string())]
    );
}

#[test]
fn ns_import_retains_relative_dependency_pattern() {
    let m = lower_to_ir("namespace import foo::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_owned(), "foo::*".to_owned())]
    );
}

#[test]
fn ns_import_dependencies_preserve_selected_literal_patterns_and_order() {
    let m = lower_to_ir("namespace import foo::* ::* ::::* ::foo::* foo::*", &reg());
    assert_eq!(
        m.namespace_imports,
        ["foo::*", "::*", "::::*", "::foo::*", "foo::*"]
            .into_iter()
            .map(|pattern| ("::".to_owned(), pattern.to_owned()))
            .collect::<Vec<_>>()
    );
    let shadowed = lower_to_ir(
        "proc namespace args {}; namespace import foo::* ::foo::*",
        &reg(),
    );
    assert_eq!(
        shadowed.namespace_imports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn ns_import_handles_force_flag() {
    // ``-force`` is the documented option; the next word is
    // the pattern.
    let m = lower_to_ir("namespace import -force ::tcltest::*", &reg());
    assert_eq!(
        m.namespace_imports,
        vec![("::".to_string(), "::tcltest::*".to_string())]
    );
}

#[test]
fn proc_dynamic_name_resolved_via_const_map() {
    // ``set name {Verbose}; proc \$name {} { ... }``
    // inside a proc — ``name`` is in the const-map, so the
    // inner proc registers as ``::Verbose``.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n proc $name {} { puts hi } }",
        &reg(),
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::Verbose").is_some(),
        "expected ::Verbose in {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn proc_dynamic_name_no_binding_stays_barrier() {
    // ``proc \$name`` with no const-map entry — bail to barrier.
    let m = lower_to_ir("proc factory {} { proc $name {} { puts hi } }", &reg());
    let factory = fixture_id_if_present(&m.procedures, "::factory")
        .and_then(|id| m.procedures.get(&id))
        .expect("factory registered");
    let last = factory.body.statements.last().expect("body");
    assert!(
        matches!(last, Statement::Barrier { .. }),
        "expected Barrier, got {last:?}"
    );
}

#[test]
fn proc_with_trailing_colons_routes_through_barrier() {
    // Tcl 9 ``proc ::ns:: {args} {body}`` registers a command
    // named ``""`` inside ``::ns``.  Static normalisation would
    // strip the trailing ``::`` and register the proc under the
    // namespace name itself — wrong cmd.  Route through
    // Statement::Barrier so the runtime ``proc`` builtin handles
    // the registration.
    let m = lower_to_ir("proc ::ns:: {a b} { puts $a$b }", &reg());
    // No procedure entry should have been AOT-registered under
    // ``::ns`` (which is what the bug would produce) or any
    // empty-key variant.
    assert!(
        fixture_id_if_present(&m.procedures, "::ns").is_none()
            && fixture_id_if_present(&m.procedures, "::ns::").is_none()
            && fixture_id_if_present(&m.procedures, "").is_none(),
        "trailing-:: proc should not be AOT-registered; got {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    // The top-level statement should be a Barrier with the
    // empty-simple-name reason.
    let top = m
        .top_level
        .statements
        .last()
        .expect("at least one top-level statement");
    match top {
        Statement::Barrier {
            reason, command, ..
        } => {
            assert_eq!(command, "proc");
            assert!(
                reason.contains("empty-simple-name"),
                "unexpected barrier reason: {reason}"
            );
        }
        other => panic!("expected Statement::Barrier, got {other:?}"),
    }
}

#[test]
fn proc_with_command_substitution_in_name_stays_barrier() {
    // ``proc \$name[suffix]`` — multi-token name with a command
    // substitution. The const-map gate only covers single-VAR
    // tokens; this stays on the runtime path.
    let m = lower_to_ir(
        "proc factory {} { set name {x}\n proc $name[suffix] {} { puts hi } }",
        &reg(),
    );
    // ``::factory`` registered; the inner proc must NOT be
    // registered under any literal name (``::x...``).
    assert!(
        !m.procedures
            .values()
            .any(|body| body.qualified_name.contains("suffix"))
    );
}

#[test]
fn proc_subst_nocommands_body_materialised() {
    // ``proc \$name {x} [subst -nocommands {return \$default}]``
    // with both ``name`` and ``default`` const-tracked materialises
    // the body to ``return 0`` and lowers it as a real script.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nocommands {return $default}] }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Verbose")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Verbose registered");
    // Body should contain a Return statement (or at least be
    // non-empty — the materialised body lowers to a real
    // statement, not a Barrier).
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
}

#[test]
fn foreach_line_lowers_to_foreach_statement() {
    // `foreachLine var filename body` lowers
    // to a single-iterator Statement::Foreach so variables assigned
    // inside the body propagate to the enclosing scope.  The
    // generic stdlib-proc path would route through Statement::Barrier and
    // lose that lattice information.
    let m = lower_to_ir(
        "foreachLine line /etc/hosts { set count [expr {$count + 1}] }",
        &reg(),
    );
    let stmts = &m.top_level.statements;
    let last = stmts.last().expect("at least one statement");
    match last {
        Statement::Foreach {
            iterators, is_lmap, ..
        } => {
            assert!(!is_lmap);
            assert_eq!(iterators.len(), 1);
            assert_eq!(iterators[0].vars, vec!["line".to_owned()]);
        }
        other => panic!("expected Statement::Foreach, got {other:?}"),
    }
}

#[test]
fn foreach_line_var_body_routes_through_barrier() {
    // `foreachLine line file $body` — the body is a Var token,
    // which is single-token but NOT a braced literal.  Compiling
    // the literal `$body` text as a static loop body would
    // produce incorrect IR / data-flow; the lowering must fall
    // through to the runtime command via Barrier.
    let m = lower_to_ir(
        "set body [gets stdin]\nforeachLine line /etc/hosts $body",
        &reg(),
    );
    let last = m.top_level.statements.last().expect("top-level statement");
    match last {
        Statement::Barrier {
            reason, command, ..
        } => {
            assert_eq!(command, "foreachLine");
            assert!(
                reason.contains("dynamic body"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected Barrier for Var body, got {other:?}"),
    }
}

#[test]
fn foreach_line_cmd_body_routes_through_barrier() {
    // Same guard for a `[cmd]` body — single-token but a Cmd
    // substitution, not a braced literal.
    let m = lower_to_ir("foreachLine line /etc/hosts [build-body]", &reg());
    let last = m.top_level.statements.last().expect("top-level statement");
    assert_runtime_opaque_call(last, &reg(), "foreachLine");
}

#[test]
fn proc_var_body_const_map_materialised() {
    // A bare `$body` body word whose value
    // is in the const-map materialises into the Procedure body
    // (matching the `proc $name {x} $body` shape).
    let m = lower_to_ir(
        "proc factory {} { set name {Greeter}\n set body {return hi}\n proc $name {} $body }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Greeter")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Greeter registered");
    // The body was const-resolved to "return hi"; lowering it
    // produces a Return statement.
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
}

#[test]
fn proc_var_body_unresolved_keeps_empty_body() {
    // `$body` has no const-map binding but `$name` does — the
    // proc name resolves, but the body stays empty (the runtime
    // proc call below carries the dynamic body text).
    let m = lower_to_ir(
        "proc factory {} { set name {Greeter}\n proc $name {} $body }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Greeter")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Greeter registered");
    assert_eq!(
        inner.body.statements.len(),
        0,
        "expected empty body for unresolved dynamic body, got {:?}",
        inner.body.statements
    );
}

#[test]
fn proc_dynamic_params_routes_through_barrier() {
    // A dynamic params word — there's no static arg-list to
    // build a real Procedure from, so the whole proc shape
    // bails to a Barrier.  Even when the name resolves via the
    // const-map.
    let m = lower_to_ir(
        "proc factory {} { set name {x}\n proc $name $params { puts hi } }",
        &reg(),
    );
    // No inner procedure should be registered — params couldn't
    // be parsed statically.
    assert!(
        fixture_id_if_present(&m.procedures, "::x").is_none(),
        "dynamic-params shape must not AOT-register: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn proc_multi_token_body_word_is_dynamic() {
    // `proc foo {} "pre${body}post"` — the body is a quoted
    // multi-token word.  Even though the first token is Str,
    // single_token_word is false so the whole word is dynamic
    // and we route through Barrier (the multi-token guard).
    let m = lower_to_ir(
        r#"proc factory {} { set body {return hi}; proc inner {} "pre${body}post" }"#,
        &reg(),
    );
    // `::inner` would only be registered if the body was
    // materialised — which can't happen for multi-token words.
    // We allow either no entry or an empty-body entry; the key
    // assertion is that the literal source text isn't compiled
    // as a script.
    if let Some(inner) =
        fixture_id_if_present(&m.procedures, "::inner").and_then(|id| m.procedures.get(&id))
    {
        assert_eq!(
            inner.body.statements.len(),
            0,
            "multi-token body must not be statically lowered, got {:?}",
            inner.body.statements
        );
    }
}

#[test]
fn proc_subst_nocommands_missing_var_skips_materialisation() {
    // ``\$default`` is not in the const-map — the materialiser
    // refuses, leaving the body to fall back to runtime
    // dispatch.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n proc $name {x} [subst -nocommands {return $default}] }",
        &reg(),
    );
    // Verbose not registered (because \$default missing means
    // we keep the dynamic body which routes via Barrier — but
    // the proc name itself was substituted). Actually
    // the proc IS registered with whatever the body lowering
    // produces; the assertion is that it's NOT the materialised
    // form.
    let inner =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id));
    if let Some(p) = inner {
        // The body should not contain a Return whose value is
        // the literal "0" — that would be the materialised
        // form. It can be empty / contain a Barrier.
        // Conservative check: just verify the helper refused.
        // (The body might still lower the original CMD-token
        // text as a runtime call.)
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                )
        );
    }
}

/// Tcl 9.1's positive family names the same effect set `-nocommands`
/// leaves — variables and backslashes on, commands off — so the body
/// materialises the same way.
///
/// tclsh 9.1b0, with `name` set to `world`:
/// `subst -variables -backslashes {hello $name\n[format X]}` → `hello
/// world`, a newline, then the untouched `[format X]`.
#[test]
fn proc_subst_tcl91_positive_switches_materialised() {
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -variables -backslashes {return $default}] }",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::Verbose")
        .and_then(|id| m.procedures.get(&id))
        .expect("::Verbose registered");
    assert!(
        !inner.body.statements.is_empty(),
        "expected lowered body, got empty"
    );
    assert!(
        !matches!(inner.body.statements[0], Statement::Barrier { .. }),
        "expected a materialised body, got {:?}",
        inner.body.statements
    );
}

/// A computed switch word leaves the call unreadable, so the registry
/// answers every kind and the materialiser refuses.
#[test]
fn proc_subst_computed_switch_word_refused() {
    let m = lower_to_ir(
        "proc factory {opt} { set name {Verbose}\n set default {0}\n proc $name {x} [subst $opt {return $default}] }",
        &reg(),
    );
    if let Some(p) =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id))
    {
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                ),
            "expected no materialised body, got {:?}",
            p.body.statements
        );
    }
}

#[test]
fn proc_subst_nocommands_nobackslashes_refused() {
    // ``-nobackslashes`` flag — semantics differ from our
    // evaluator's default. Refuse and fall through.
    let m = lower_to_ir(
        "proc factory {} { set name {Verbose}\n set default {0}\n proc $name {x} [subst -nobackslashes -nocommands {return $default}] }",
        &reg(),
    );
    let inner =
        fixture_id_if_present(&m.procedures, "::Verbose").and_then(|id| m.procedures.get(&id));
    if let Some(p) = inner {
        // Body did not materialise — should be empty or have
        // a fallback shape.
        assert!(
            p.body.statements.is_empty()
                || matches!(
                    p.body.statements[0],
                    Statement::Call { .. } | Statement::Barrier { .. }
                )
        );
    }
}

#[test]
fn proc_dynamic_name_picks_latest_set() {
    // Multiple ``set`` calls — the most recent literal wins.
    let m = lower_to_ir(
        "proc factory {} { set name {First}\n set name {Second}\n proc $name {} { puts hi } }",
        &reg(),
    );
    assert!(fixture_id_if_present(&m.procedures, "::Second").is_some());
    assert!(fixture_id_if_present(&m.procedures, "::First").is_none());
}

#[test]
fn ns_export_recorded() {
    let m = lower_to_ir("namespace eval ::tcltest { namespace export test }", &reg());
    // The export was inside a ``namespace eval`` body so the
    // context namespace is ``::tcltest``.
    assert!(
        m.namespace_exports
            .iter()
            .any(|(ns, pat)| ns == "::tcltest" && pat == "test")
    );
}

#[test]
fn namespace_directive_metadata_keeps_actual_availability_and_source_config() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut descriptor = registry.get("namespace").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands())),
    );
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let make = |context: std::sync::Arc<tcl_registry::model::ContextRegistry>| {
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let mut lowerer =
            Lowerer::with_config(current.commands(), config).with_dialect(Some(profile));
        lowerer.set_source_analysis_options(entry.options());
        lowerer.with_resolved_analysis_input(input)
    };
    let source = "namespace export exposed";
    let mut selected = make(std::sync::Arc::clone(&current));
    selected.lower(source);
    assert_eq!(
        selected.namespace_exports,
        [("::".to_owned(), "exposed".to_owned())]
    );
    let mut unavailable = make(older);
    unavailable.lower(source);
    assert_eq!(
        unavailable.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut missing = make(std::sync::Arc::clone(&current));
    missing.metadata_input = None;
    missing.dialect_context = None;
    missing.lower(source);
    assert_eq!(
        missing.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
    let logical_input = crate::analyser::ResolvedAnalysisInput::new(
        logical_profile,
        logical_profile,
        std::sync::Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
    );
    let mut missing_authoring =
        Lowerer::with_config(current.commands(), logical_input.lexer_config())
            .with_resolved_analysis_input(logical_input);
    assert!(missing_authoring.invocation_metadata_context().is_some());
    assert_eq!(
        missing_authoring.source_entry_origin,
        SourceEntryOrigin::Authoring
    );
    missing_authoring.metadata_input = None;
    missing_authoring.logical_source_input = None;
    missing_authoring.dialect_context = None;
    assert!(!missing_authoring.allows_standalone_registry_metadata());
    assert!(missing_authoring.invocation_metadata_context().is_none());
    missing_authoring.lower(source);
    assert_eq!(
        missing_authoring.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut stale = make(std::sync::Arc::clone(&current));
    stale.config.strict_quoting = !config.strict_quoting;
    stale.lower(source);
    assert_eq!(
        stale.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
    let mut foreign = make(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    foreign.lower(source);
    assert_eq!(
        foreign.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn namespace_directive_metadata_uses_frozen_handler_operands() {
    for source in [
        "set pattern {-c}; namespace export OLD; namespace export -clear $pattern -- x",
        "rename namespace ns; set pattern {-c}; ns export OLD; ns export -clear $pattern -- x",
    ] {
        let module = lower_to_ir(source, &reg());
        assert_eq!(
            module.namespace_exports,
            ["OLD", "-c", "--", "x"].map(|pattern| ("::".to_owned(), pattern.to_owned())),
            "{source}",
        );
    }
    let replaced = lower_to_ir("proc namespace args {}; namespace export x", &reg());
    assert_eq!(
        replaced.namespace_exports,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn ns_import_in_dead_branch_suppressed() {
    // ``if {0} { namespace import ::evil::* }`` — the
    // import is inside a syntactically-dead branch so it must
    // NOT be recorded.
    let m = lower_to_ir("if {0} { namespace import ::evil::* }", &reg());
    assert_eq!(
        m.namespace_imports.len(),
        0,
        "imports inside dead if{{0}} branch must not be collected, got {:?}",
        m.namespace_imports,
    );
}

#[test]
fn ns_import_in_static_true_else_suppressed() {
    // ``if {1} { ... } else { namespace import ::evil::* }``
    // — the else branch is dead.
    let m = lower_to_ir(
        "if {1} { namespace import ::good::* } else { namespace import ::evil::* }",
        &reg(),
    );
    // Only ``::good::*`` recorded.
    assert_eq!(m.namespace_imports.len(), 1);
    assert_eq!(m.namespace_imports[0].1, "::good::*");
}

#[test]
fn const_prop_inherited_into_catch_body() {
    // Child scope (catch body) inherits parent's const-map.
    let m = lower_to_ir(
        "proc f {} { set body {set x 1}\n catch { uplevel 1 $body } }",
        &reg(),
    );
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    // Look for a Catch wrapping an UpFrame.
    let catch_stmt = proc
        .body
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Catch { .. }))
        .expect("expected Catch");
    if let Statement::Catch { body, .. } = catch_stmt {
        assert!(
            body.statements
                .iter()
                .any(|s| matches!(s, Statement::UpFrame { .. })),
            "expected UpFrame inside catch body, got {body:?}",
        );
    }
}

// trace add execution module-fact population

#[test]
fn trace_add_execution_literal_recorded() {
    let m = lower_to_ir("trace add execution foo enter handler", &reg());
    assert!(m.traced_commands.contains("foo"));
    assert!(!m.has_dynamic_trace);
}

#[test]
fn trace_add_execution_dynamic_widens() {
    let m = lower_to_ir("trace add execution $cmd enter handler", &reg());
    assert!(m.has_dynamic_trace);
    assert_eq!(m.traced_commands.len(), 0);
}

#[test]
fn trace_add_execution_qualified_canonicalised() {
    let m = lower_to_ir("trace add execution ::ns::foo enter h", &reg());
    // Stripped of leading ``::`` so the GVN gate's
    // `command.trim_start_matches("::")` lookup hits.
    assert!(m.traced_commands.contains("ns::foo"));
}

#[test]
fn trace_add_variable_does_not_record_execution_trace() {
    // `trace add variable` is a separate channel — should not
    // populate `traced_commands` (those are command traces only).
    let m = lower_to_ir("trace add variable x write h", &reg());
    assert_eq!(m.traced_commands.len(), 0);
    assert!(!m.has_dynamic_trace);
}

/// `walk_for_trace`'s post-lower scan recurses once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame`
/// body, with no depth cap of its own. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH`
/// (256) by `lower_script`/`lower_body` (this same `lower_to_ir` call
/// builds the `Script` `walk_for_trace` then scans), so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// source nesting is comfortably past the cap; the assertion is
/// that lowering (which runs `populate_trace_facts` ->
/// `walk_for_trace`) returns at all, not what it returns. Spawns its
/// own big-stack thread since the lexer/CST/segmenter stages upstream
/// of `lower_script`'s own cap still walk the full un-truncated source
/// nesting before that cap trims it — same rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_walk_for_trace() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("trace add execution foo enter handler\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = lower_to_ir(&src, &reg());
        })
        .unwrap()
        .join()
        .unwrap();
}

// trace add/remove/variable/vdelete module-fact population

#[test]
fn trace_target_names_keep_empty_and_literal_sigil_data() {
    let registry = reg();
    let empty = lower_to_ir("trace add variable {} write callback", &registry);
    assert!(empty.traced_variables.contains(""));
    assert!(!empty.traced_variables.contains("$"));
    let literal = lower_to_ir("trace add variable {$counter} write callback", &registry);
    assert!(literal.traced_variables.contains("$counter"));
    assert!(!literal.traced_variables.contains("counter"));
}

#[test]
fn trace_add_variable_literal_recorded() {
    let m = lower_to_ir("trace add variable x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_remove_variable_literal_recorded() {
    // A `remove` only proves a trace *might* have existed — still
    // conservative to record, mirroring `trace add`.
    let m = lower_to_ir("trace remove variable x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_qualified_canonicalised() {
    let m = lower_to_ir("trace add variable ::x write h", &reg());
    // `::`-stripped so a top-level `set x 5` (chain key `x`) matches.
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_dynamic_widens() {
    let m = lower_to_ir("trace add variable $name write h", &reg());
    assert!(m.has_dynamic_variable_trace);
    assert_eq!(m.traced_variables.len(), 0);
}

#[test]
fn trace_add_variable_inside_proc_recorded() {
    let m = lower_to_ir(
        "proc setup {} { trace add variable ::x read onread }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_method_recorded() {
    // TclOO method bodies are extracted into `module.methods` (a
    // separate map from `module.procedures`) before this scan runs —
    // must not be missed just because it isn't a plain proc.
    let m = lower_to_ir(
        "oo::class create C {\n method m {} { trace add variable v write cb }\n}",
        &reg(),
    );
    assert!(m.traced_variables.contains("v"), "{:?}", m.traced_variables);
}

#[test]
fn trace_legacy_variable_form_recorded() {
    // The deprecated `trace variable name ops command` spelling must
    // populate the same fact as `trace add variable` — no
    // hardcoded-per-form gap.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let m = lower_to_ir("trace variable x r onread", registry);
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_legacy_vdelete_form_recorded() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let m = lower_to_ir("trace vdelete x r onread", registry);
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn removed_legacy_trace_forms_do_not_donate_tcl9_transitions() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in ["trace variable x r onread", "trace vdelete x r onread"] {
        let module = lower_to_ir(source, registry);
        assert!(module.traced_variables.is_empty());
        assert!(!module.has_dynamic_variable_trace);
    }
}

#[test]
fn trace_info_variable_does_not_record() {
    // `trace info`/`vinfo` only query trace state — no
    // `ESTABLISHES_VARIABLE_TRACE` trait, so they must not widen the
    // module fact.
    let m = lower_to_ir("trace info variable x", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_add_variable_through_interp_alias_recorded() {
    // `interp alias {} tracer {} trace` means `tracer add variable ...`
    // is really a `trace add variable ...` call — the whole-module scan
    // must key off the canonical (alias-resolved) command name, not the
    // source-surface `tracer` spelling, or this trace target is missed.
    let m = lower_to_ir(
        "interp alias {} tracer {} trace\ntracer add variable x write h",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_execution_through_interp_alias_recorded() {
    // Same alias-resolution requirement for the execution-trace channel:
    // `walk_for_trace` gates entry into both `traced_commands` and
    // `populate_variable_trace_facts` on one shared canonical-name check.
    let m = lower_to_ir(
        "interp alias {} tracer {} trace\ntracer add execution foo enter h",
        &reg(),
    );
    assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
    assert!(!m.has_dynamic_trace);
}

#[test]
fn trace_add_execution_does_not_record_variable_trace() {
    // The command-execution channel is separate — should not
    // populate `traced_variables`.
    let m = lower_to_ir("trace add execution foo enter h", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(!m.has_dynamic_variable_trace);
}

#[test]
fn trace_add_execution_inside_proc_recorded() {
    let m = lower_to_ir("proc init {} { trace add execution foo enter h }", &reg());
    assert!(
        m.traced_commands.contains("foo"),
        "traced_commands={:?}",
        m.traced_commands,
    );
}

#[test]
fn trace_add_variable_abbreviated_type_word_recorded() {
    // `trace add v x write h` / `trace add var x write h` install the
    // same variable trace as the full `variable` spelling — C Tcl
    // accepts any unique prefix (checked against tclsh 8.6.14).
    let m = lower_to_ir("trace add v x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
    let m = lower_to_ir("trace add var x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_remove_variable_abbreviated_type_word_recorded() {
    let m = lower_to_ir("trace remove var x write h", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_execution_abbreviated_type_word_not_variable() {
    // `e`/`exec` abbreviate `execution`, not `variable` — must not
    // widen `traced_variables`.
    let m = lower_to_ir("trace add e foo enter h", &reg());
    assert_eq!(m.traced_variables.len(), 0);
    assert!(m.traced_commands.contains("foo"), "{:?}", m.traced_commands);
}

#[test]
fn trace_add_variable_inside_namespace_eval_body_recorded() {
    // A trace installed inside a `namespace eval` body (a synthetic
    // `Module::body_units` entry, not a named proc) must populate the
    // same whole-module fact as one inside a `proc`.
    let m = lower_to_ir(
        "namespace eval ::n { trace add variable ::x write h }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_apply_body_recorded() {
    let m = lower_to_ir("apply {{} { trace add variable ::x write h }}", &reg());
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

#[test]
fn trace_add_variable_inside_method_body_recorded() {
    let m = lower_to_ir(
        "oo::class create C { method m {} { trace add variable ::x write h } }",
        &reg(),
    );
    assert!(m.traced_variables.contains("x"), "{:?}", m.traced_variables);
}

// barrier-gate

#[test]
fn body_has_dynamic_barrier_clean() {
    // No barriers at all.
    assert!(!body_has_dynamic_barrier(
        "set x 1",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Barrier with a fully literal body.
    assert!(!body_has_dynamic_barrier(
        "eval { set x 1 }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Nested literal.
    assert!(!body_has_dynamic_barrier(
        "if { 1 } { eval { set x 1 } }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn descriptor_binding_requirement_keeps_original_head_and_exact_metadata_without_native_receipt() {
    let mut registry = reg();
    let prototype = registry.get_exact("list").unwrap().clone();
    for label in ["::probe", ":probe", "A:::probe", "probe::", ""] {
        let mut descriptor = prototype.clone();
        descriptor.name = label;
        registry.insert(descriptor);
    }
    for label in ["::probe", ":probe", "A:::probe", "probe::", ""] {
        let source = if label.is_empty() {
            "{} value".to_owned()
        } else {
            format!("{label} value")
        };
        let mut lowerer = Lowerer::new(&registry);
        lowerer.word_space = WordSpace::new(&source, 0, lowerer.source_channel);
        lowerer.command_binding_site_stack.push(Vec::new());
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(&source, 0, lowerer.config);
        let key = registry.get_exact(label).unwrap().descriptor_key();
        lowerer.record_consumed_command_binding(
            &commands[0],
            "::",
            crate::registry_invocation::SelectedLoweringBindingIdentity::Logical(key),
        );
        let site = &lowerer.command_binding_site_stack[0][0];
        assert_eq!(site.binding.name, label);
        assert_eq!(
            site.binding.identity,
            tcl_syntax::naming::unroot_rooted_key(label).unwrap_or(label)
        );
        assert!(site.binding.namespace_context.is_none());
        assert!(
            site.source_tokens
                .as_deref()
                .unwrap()
                .source_binding
                .is_none()
        );
    }
}

#[test]
fn unavailable_lowering_metadata_keeps_generic_dispatch_and_conservative_method_names() {
    let registry = reg();
    let source = "unresolved local $dynamic";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.source_entry_origin = SourceEntryOrigin::Driver;
    lowerer.metadata_origin = LoweringMetadataOrigin::Supplied;
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let statement = lowerer.lower_default(&commands[0], "::");
    let Statement::Call {
        canonical_command,
        defs,
        reads,
        ..
    } = &statement
    else {
        panic!("missing original/context ownership must retain a generic call");
    };
    assert!(canonical_command.is_none());
    assert_eq!(defs.len(), 0);
    assert_eq!(reads.len(), 0);
    assert_eq!(
        lowerer.advisory_method_instance_names(&statement),
        vec!["local", "${dynamic}"]
    );
}

#[test]
fn method_variable_advice_preserves_literal_names_and_captured_prefix_values() {
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry).trace_visible();
    lowerer.lower("variable {}; variable {$counter}; variable {brace{name}}; variable {-dash}");
    let names: Vec<_> = lowerer
        .module
        .top_level
        .statements
        .iter()
        .flat_map(|statement| lowerer.advisory_method_instance_names(statement))
        .collect();
    assert_eq!(names, vec!["", "$counter", "brace{name}", "-dash"]);

    let mut captured = Lowerer::new(&registry).trace_visible();
    captured.lower("interp alias {} ivar {} variable {$counter}; ivar");
    let statement = captured.module.top_level.statements.last().unwrap();
    assert_eq!(
        captured.advisory_method_instance_names(statement),
        vec!["$counter"]
    );
}

#[test]
fn unresolved_method_name_protection_keeps_original_source_text_without_reparsing() {
    let registry = reg();
    let source = "unresolved {} {$counter} {brace{name}} {a(b)} {-dash} $dynamic";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.source_entry_origin = SourceEntryOrigin::Driver;
    lowerer.metadata_origin = LoweringMetadataOrigin::Supplied;
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let statement = lowerer.lower_default(&commands[0], "::");
    let Statement::Call { args, .. } = &statement else {
        panic!("generic source fallback");
    };
    assert_eq!(lowerer.advisory_method_instance_names(&statement), *args);
    assert_eq!(
        args,
        &vec!["", "$counter", "brace{name}", "a(b)", "-dash", "${dynamic}"]
    );
}

#[test]
fn local_body_literal_reference_lookup_uses_whole_scalar_source_geometry() {
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry);
    lowerer.proc_depth = 1;
    lowerer.const_map_stack.push(HashMap::from([(
        "body".to_owned(),
        "return original".to_owned(),
    )]));
    for reference in ["$body", "${body}"] {
        assert_eq!(
            lowerer.const_map_lookup(reference).as_deref(),
            Some("return original")
        );
    }
    for reference in [
        "body",
        "$body suffix",
        "${body}suffix",
        "$body(index)",
        "${body(index)}",
        "$",
        "${body",
        "$::body",
    ] {
        assert_eq!(lowerer.const_map_lookup(reference), None, "{reference}");
    }
    lowerer.proc_depth = 0;
    assert_eq!(lowerer.const_map_lookup("$body"), None);
}

#[test]
fn local_body_literal_retention_requires_actual_assignment_and_decodes_the_written_key() {
    let config = tcl_lexer::LexerConfig::default();
    let source = r"set b\x6fdy {return original}";
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let seg = &commands[0];
    let assignment = Statement::AssignConst {
        span: seg.span,
        name: seg.args()[0].clone(),
        name_braced: false,
        value: seg.args()[1].clone(),
        value_span: None,
    };
    assert_eq!(
        assigned_local_body_literal(seg, &assignment, config),
        Some(("body".to_owned(), "return original".to_owned()))
    );
    let opaque = Statement::Barrier {
        span: seg.span,
        reason: "opaque invocation".to_owned(),
        command: "set".to_owned(),
        canonical_command: None,
        args: seg.args().to_vec(),
        tokens: None,
    };
    assert_eq!(assigned_local_body_literal(seg, &opaque, config), None);
}

#[test]
fn local_body_literal_invalidation_distinguishes_literal_sigils_and_decodes_write_keys() {
    let config = tcl_lexer::LexerConfig::default();
    let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
    let write = |name: &str, name_braced: bool| Statement::AssignConst {
        span: tcl_lexer::Span::new(0, 0),
        name: name.to_owned(),
        name_braced,
        value: "replacement".to_owned(),
        value_span: None,
    };
    invalidate_const_map_for(&write("$body", true), &mut scope, config);
    assert!(scope.contains_key("body"));
    invalidate_const_map_for(&write("::body", true), &mut scope, config);
    assert!(scope.contains_key("body"));
    invalidate_const_map_for(&write(r"b\x6fdy", false), &mut scope, config);
    assert!(scope.is_empty());
    scope.insert("body".to_owned(), "return original".to_owned());
    invalidate_const_map_for(&write("body(index)", true), &mut scope, config);
    assert!(scope.is_empty());
    scope.insert("body".to_owned(), "return original".to_owned());
    invalidate_const_map_for(&write("$computed", false), &mut scope, config);
    assert!(scope.is_empty());
}

#[test]
fn local_body_literal_generic_call_with_empty_defs_withdraws_all_retained_values() {
    let config = tcl_lexer::LexerConfig::default();
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry);
    let source = "unknown_writer $computed replacement";
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let call = lowerer.lower_default(&commands[0], "::");
    assert!(matches!(&call, Statement::Call { defs, .. } if defs.is_empty()));
    let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
    invalidate_const_map_for(&call, &mut scope, config);
    assert!(scope.is_empty());
}

#[test]
fn local_body_literal_quiet_logical_source_retains_factory_names_and_bodies() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "proc factory {} {set name {LiteralChild}; set body {return original}; proc $name {} $body}";
    let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input);
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let child = fixture_id_if_present(&module.procedures, "::LiteralChild")
        .and_then(|id| module.procedures.get(&id))
        .expect("genuine quiet Logical source retains the original child");
    assert_eq!(child.body_source.as_deref(), Some("return original"));
    assert!(module.source_entry.native_entry.is_none());
}

#[test]
fn local_body_literal_opcode_without_original_quiet_source_owner_cannot_retain_contents() {
    let registry = reg();
    let source = "set body {return original}";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.proc_depth = 1;
    lowerer.const_map_stack.push(HashMap::from([(
        "previous".to_owned(),
        "return stale".to_owned(),
    )]));
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let seg = &commands[0];
    let synthetic = Statement::AssignConst {
        span: seg.span,
        name: seg.args()[0].clone(),
        name_braced: false,
        value: seg.args()[1].clone(),
        value_span: None,
    };
    lowerer.update_const_map(seg, &synthetic);
    assert!(lowerer.const_map_stack.last().unwrap().is_empty());
    assert_eq!(lowerer.original_native_written_argument_value(seg, 1), None);
}

#[test]
fn local_body_literal_expression_and_value_evaluation_withdraw_unclosed_effects() {
    let registry = reg();
    let config = tcl_lexer::LexerConfig::default();
    for source in [
        "expr {[overwrite]}",
        "expr {$watched}",
        "set other [overwrite]",
        "set other $watched",
        "incr other $watched",
    ] {
        let module = lower_to_ir(source, &registry);
        let statement = module
            .top_level
            .statements
            .first()
            .expect("original statement");
        assert!(!matches!(statement, Statement::AssignConst { .. }));
        let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
        invalidate_const_map_for(statement, &mut scope, config);
        assert!(scope.is_empty(), "unclosed effects: {source}");
    }
}

#[test]
fn class_variable_source_union_keeps_decoded_names_and_slot_operation_boundaries() {
    let registry = reg();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let source = r"variable -set {} {$counter} {brace{name}} {-dash}; variable \$counter";
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let names = declared_member_vars(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &segments,
        Some(profile.surface_query()),
        config,
    );
    assert_eq!(
        names,
        HashSet::from([
            String::new(),
            "$counter".to_owned(),
            "brace{name}".to_owned(),
            "-dash".to_owned()
        ])
    );
    assert!(!names.contains("counter"));
    assert!(!names.contains("-set"));
    let escaped_segments =
        crate::segmenter::segment_commands_with_offset_and_config(r"variable \$counter", 0, config);
    assert_eq!(
        declared_member_vars(
            &tcl_registry::definer::TCLOO_GRAMMAR,
            &escaped_segments,
            Some(profile.surface_query()),
            config,
        ),
        HashSet::from(["$counter".to_owned()])
    );
}

#[test]
fn class_variable_unknown_operands_keep_known_literal_protection_without_reference_parsing() {
    let registry = reg();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        "variable $dynamic {} {$counter} {a(b)}",
        0,
        config,
    );
    let names = declared_member_vars(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &segments,
        Some(profile.surface_query()),
        config,
    );
    assert_eq!(
        names,
        HashSet::from([String::new(), "$counter".to_owned(), "a(b)".to_owned()])
    );
    assert!(!names.contains("counter"));
    assert!(!names.contains("a"));
    assert!(!names.contains("dynamic"));
}

#[test]
fn body_barrier_metadata_keeps_exact_selected_rooted_descriptor() {
    let mut registry = reg();
    assert!(body_has_dynamic_barrier(
        "::eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    let mut rooted = registry.get_exact("list").unwrap().clone();
    rooted.name = "::eval";
    registry.insert(rooted);
    assert!(!body_has_dynamic_barrier(
        "::eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "eval $body",
        &registry,
        tcl_lexer::LexerConfig::default()
    ));
    for head in ["::::eval", ":eval", "eval::", "eval:::member"] {
        assert!(!body_has_dynamic_barrier(
            &format!("{head} $body"),
            &registry,
            tcl_lexer::LexerConfig::default()
        ));
    }
}

#[test]
fn body_has_dynamic_barrier_dynamic_eval_body() {
    // ``eval $x`` inside the outer body — dynamic.
    assert!(body_has_dynamic_barrier(
        "eval $x",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // Same shape nested.
    assert!(body_has_dynamic_barrier(
        "if { 1 } { eval $x }",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_dynamic_uplevel_body() {
    assert!(body_has_dynamic_barrier(
        "uplevel 1 $body",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "uplevel #0 $b",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_requires_proved_uplevel_level() {
    // A numeric value selects a frame, but "marker" is the first script
    // word on every pinned C release. Bracing the last word cannot decide
    // which operation an unknown first value requests.
    for source in ["uplevel 0 {set x 1}", "uplevel #0 {set x 1}"] {
        assert!(!body_has_dynamic_barrier(
            source,
            &reg(),
            tcl_lexer::LexerConfig::default()
        ));
    }
    assert!(body_has_dynamic_barrier(
        "uplevel $lvl {set x 1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_has_dynamic_barrier_qualified_eval_uplevel() {
    // ``::eval`` and ``::uplevel`` are also caught.
    assert!(body_has_dynamic_barrier(
        "::eval $x",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "::uplevel 1 $body",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

/// The gate resolves `uplevel`'s script word through the
/// registry's `ArgRole::Body` resolver, not a local level-word sniff, so
/// every documented `uplevel ?level? arg ?arg ...?` shape agrees with
/// `uplevel_body_arg_role_skips_optional_level` in `tcl-registry`.
#[test]
fn body_has_dynamic_barrier_uplevel_level_shapes_follow_the_registry() {
    let r = reg();
    // TN — a literal script in each level spelling relaxes.
    for clean in [
        "uplevel {set x 1}",
        "uplevel 1 {set x 1}",
        "uplevel #0 {set x 1}",
        "uplevel 0 {set x 1}",
    ] {
        assert!(
            !body_has_dynamic_barrier(clean, &r, tcl_lexer::LexerConfig::default()),
            "{clean} must not poison"
        );
    }
    // TP — a substituted script word in each level spelling poisons.
    for dirty in [
        "uplevel $body",
        "uplevel 1 $body",
        "uplevel #0 $body",
        "uplevel $lvl $body",
        "uplevel $lvl {set x 1}",
        "uplevel [expr {$n - 1}] {set x 1}",
        "uplevel 1 [gen]",
    ] {
        assert!(
            body_has_dynamic_barrier(dirty, &r, tcl_lexer::LexerConfig::default()),
            "{dirty} must poison"
        );
    }
    // FN guard — the words after the script word concatenate into it
    // (`SCRIPT_CONCATENATES_ARGS`), so a dynamic tail is still dynamic
    // even though the marked body word is a braced literal.
    assert!(body_has_dynamic_barrier(
        "uplevel 1 {set x} $tail",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "eval {set x} $tail",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // …and an all-literal multi-word tail stays clean.
    assert!(!body_has_dynamic_barrier(
        "uplevel 1 {set x} {1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // A bodyless `uplevel 1` is a wrong-#args error the registry exposes
    // no body word for — poison rather than guess.
    assert!(body_has_dynamic_barrier(
        "uplevel 1",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
    // A local level sniff that strips a leading `-` before the digit test
    // would accept `-N` as a level; the registry does not, and the
    // registry is right.  On tclsh 8.6.14, with `proc -1 {args} {…}`
    // defined, `uplevel -1 {set x 1}` *calls* `-1` with `set x 1` — the
    // word is the script's first word, not a level.  tclsh9.0.4 instead
    // errors `bad level "-1"`.  Under either reading the word is an
    // unbraced script word, so the gate poisons.
    assert!(body_has_dynamic_barrier(
        "uplevel -1 {set x 1}",
        &reg(),
        tcl_lexer::LexerConfig::default()
    ));
}

/// The eval-family *subcommand*
/// members carry `EVALUATES_CODE` / `SCRIPT_CONCATENATES_ARGS` on the
/// subcommand, not on the `namespace` / `interp` spec.  Composing them
/// (`invocation_traits`) plus registry-resolved body indices makes them
/// flow through the same path a bare `eval` does, which a parent-only
/// trait test missed entirely.
#[test]
fn body_has_dynamic_barrier_covers_compound_eval_family() {
    let r = reg();
    // TP — the script is substituted.
    assert!(body_has_dynamic_barrier(
        "namespace eval ns $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "namespace inscope ns $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "interp eval slave $body",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // TN — a braced literal script relaxes, and its own contents are
    // still recursed.
    assert!(!body_has_dynamic_barrier(
        "namespace eval ns {set x 1}",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_has_dynamic_barrier(
        "namespace eval ns {eval $x}",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // TN — a `namespace` subcommand that evaluates nothing is not a
    // barrier, however dynamic its arguments are.
    assert!(!body_has_dynamic_barrier(
        "namespace delete $ns",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "namespace current",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
}

/// TN for the barrier test itself: an ordinary command with a
/// substituted argument is not a script evaluator, so it never poisons —
/// the gate keys on the registry trait, not on "has a `$var` argument".
#[test]
fn body_has_dynamic_barrier_non_body_command_is_clean() {
    let r = reg();
    assert!(!body_has_dynamic_barrier(
        "set x $y",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "puts [format %s $x]",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_has_dynamic_barrier(
        "lappend acc $item",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
    // A user command the registry does not know at all is not a barrier.
    assert!(!body_has_dynamic_barrier(
        "mycmd $script",
        &r,
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn try_lower_eval_static_rejects_nested_dynamic_barrier() {
    // The relaxer would normally promote ``eval { ... }`` to a
    // ``Statement::Block``; with the barrier gate, the nested
    // ``eval $x`` poisons the relaxation and we fall back to
    // ``Statement::Barrier``.
    let m = lower_to_ir(r"eval { eval $x }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert_runtime_opaque_call(stmt, &reg(), "eval");
}

#[test]
fn try_lower_eval_static_clean_body_relaxes() {
    // No nested barrier — relaxes to Block.
    let m = lower_to_ir("eval { set x 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    let Statement::Block { error_context, .. } = stmt else {
        panic!("expected Block (relaxed), got {stmt:?}");
    };
    assert_eq!(
        *error_context,
        Some(tcl_registry::InlineBodyErrorContext::SameFrameScriptEvaluation),
    );
}

// The next three tests confirm `eval` and `uplevel` dispatch
// through the registry-driven hook ID to the expected output.

#[test]
fn registry_dispatch_eval_static_body_lowers_to_block() {
    // `eval {body}` with a literal body should still relax
    // to `Statement::Block` when dispatched through the
    // registry-driven path.
    let m = lower_to_ir("eval { set y 2 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Block { .. }),
        "expected Block via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_uplevel_static_body_lowers_to_upframe() {
    // `uplevel 1 {body}` with a literal body should lower
    // to a UpFrame via the registry-driven path. It runs inside a
    // procedure: at the global level C raises `bad level "1"`.
    let m = lower_to_ir("proc f {} {uplevel 1 { set z 3 }}", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let stmt = proc.body.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::UpFrame { .. }),
        "expected UpFrame via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_unknown_command_falls_through_to_default() {
    // `lower_command` has no per-
    // name fallback — commands that aren't in the registry
    // (or whose `lowering_hook` is `None`) route directly
    // to [`Lowerer::lower_default`], which emits a generic
    // `Statement::Call`.  Pin that contract with a clearly
    // unknown command name.
    let m = lower_to_ir("totallyMadeUpCommand arg1 arg2", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "totallyMadeUpCommand",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
}

#[test]
fn deferred_tk_callback_does_not_hide_constructor_variable_defs() {
    let m = lower_to_ir(
        "entry .e -textvariable input -validatecommand {return 1}",
        &reg(),
    );
    let stmt = m.top_level.statements.first().expect("entry call");
    match stmt {
        Statement::Call { command, defs, .. } => {
            assert_eq!(command, "entry");
            assert_eq!(defs, &["::input"]);
        }
        other => panic!("deferred callback must not make the constructor opaque: {other:?}"),
    }
}

#[test]
fn immediate_tcltest_body_remains_a_runtime_barrier() {
    let m = lower_to_ir(
        "tcltest::test sample description -body {error failure}",
        &reg(),
    );
    let stmt = m.top_level.statements.first().expect("test call");
    assert_runtime_opaque_call(stmt, &reg(), "tcltest::test");
}

#[test]
fn same_invocation_command_prefix_is_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = lower_to_ir("lsort -command compare {b a}", &reg());
    let stmt = m.top_level.statements.first().expect("lsort call");
    assert_runtime_opaque_call(stmt, &reg(), "lsort");
}

#[test]
fn deferred_command_prefix_does_not_create_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for (source, expected_command) in [
        ("scrollbar .s -command moved", "scrollbar"),
        ("trace add variable v write changed", "trace"),
    ] {
        let m = lower_to_ir(source, &reg());
        let stmt = m.top_level.statements.first().expect("callback host call");
        assert!(
            matches!(stmt, Statement::Call { command, .. } if command == expected_command),
            "a stored command prefix cannot affect its registration invocation: {stmt:?}",
        );
    }
}

#[test]
fn complete_logical_source_keeps_original_structure_and_native_absence() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "proc subject {} {return VALUE}; set result VALUE";
    let mut current = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    current.lower(source);
    let module = current.finish_module(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_some());
    // Upstream keeps non-numeric bare literals as AssignValue so escape
    // spelling survives. This assertion covers IR/source ownership only.
    let statement = module.top_level.statements.iter().find(|statement| {
            matches!(statement, Statement::AssignValue { name, value, value_needs_backsubst: false, .. }
                if name == "result" && value == "VALUE")
        }).expect("unchanged literal assignment retains the upstream representation");
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let literal = crate::type_infer::original_literal_argument_contents(
        context.commands(),
        &module,
        tokens,
        1,
    )
    .expect("assignment retains its genuine original source");
    assert_eq!(literal.value(), "VALUE");
    assert_eq!(
        module.source_entry.logical_source_input.as_ref(),
        Some(&input)
    );
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.source_entry.execution_name_policy.is_none());

    let mut stale = Lowerer::with_config(context.commands(), input.lexer_config());
    stale.config.strict_quoting = !stale.config.strict_quoting;
    stale = stale.with_resolved_analysis_input(input.clone());
    assert!(fixture_id_if_present(&stale.lower(source).procedures, "::subject").is_none());
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        input.lexer_config(),
    );
    let mut foreign = Lowerer::with_config(context.commands(), foreign.lexer_config())
        .with_resolved_analysis_input(foreign);
    assert!(fixture_id_if_present(&foreign.lower(source).procedures, "::subject").is_none());

    let native_profile = context.commands().profile().unwrap();
    let native = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(native_profile.grammar),
    );
    // No supplied entry: a fresh tclsh defines `::subject` (user decision
    // 2026-10-19T14:20Z). Explicit unknown history still withholds it.
    let mut fresh = Lowerer::with_config(context.commands(), native.lexer_config())
        .with_resolved_analysis_input(native.clone());
    let module = fresh.lower(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_some());
    assert!(module.source_entry.logical_source_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert!(!module.source_entry.unknown_entry);
    let mut absent = Lowerer::with_config(context.commands(), native.lexer_config())
        .with_resolved_analysis_input(native)
        .with_explicit_unknown_entry();
    let module = absent.lower(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_none());
    assert!(module.source_entry.logical_source_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.source_entry.unknown_entry);
}

#[test]
fn logical_original_declaration_layout_keeps_unicode_and_qualified_alias_heads() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These exact UTF-8 source spellings exercise conditional Logical
    // layout selection, independently of any Native slot or execution.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for source in [
        "rename proc ::α; ::α subject {} {return VALUE}",
        "interp alias {} ::α {} proc; ::α subject {} {return VALUE}",
        "rename proc ::α; interp alias {} ::β {} ::α; ::β subject {} {return VALUE}",
        r"rename proc ::α; \u003a\u003aα subject {} {return VALUE}",
    ] {
        let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source);
        assert!(
            fixture_id_if_present(&module.procedures, "::subject").is_some(),
            "{source}"
        );
        assert!(module.source_entry.native_entry.is_none());
        assert!(module.source_entry.execution_name_policy.is_none());
        assert_eq!(
            module.source_entry.logical_source_input.as_ref(),
            Some(&input)
        );
    }
    for source in [
        "interp alias {} ::α {} proc subject; ::α {} {return VALUE}",
        "interp alias {} ::α {} proc subject {}; ::α {return VALUE}",
    ] {
        let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source);
        let procedure = fixture_id_if_present(&module.procedures, "::subject")
            .and_then(|id| module.procedures.get(&id))
            .expect("captured source declaration");
        assert_eq!(procedure.params, [] as [std::string::String; 0]);
        assert_eq!(procedure.params_raw, "");
        assert!(module.source_entry.native_entry.is_none());
    }
    let source = "rename proc ::α; ::α ::α args {return PAYLOAD}; ::α subject {} {}";
    let mut replaced = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input);
    assert!(fixture_id_if_present(&replaced.lower(source).procedures, "::subject").is_none());
}

#[test]
fn complete_source_input_declines_changed_unit_profile_without_authoring_fallback() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let selected = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    assert!(selected.invocation_metadata_context().is_some());
    assert_eq!(selected.logical_source_input.as_ref(), Some(&input));
    let mut wrong = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_dialect(Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()))
        .with_resolved_analysis_input(input.clone());
    assert!(wrong.invocation_metadata_context().is_none());
    assert!(wrong.unknown_entry);
    let module = wrong.lower("puts VALUE");
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert!(module.source_entry.unknown_entry);
    assert!(module.source_entry.logical_source_input.is_none());
}

#[test]
fn lowered_generic_metadata_keeps_supplied_context_and_refuses_missing_driver_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut missing = Lowerer::with_config(registry, config).with_dialect(registry.profile());
    missing.set_source_analysis_options(SourceAnalysisOptions::default());
    missing = missing.with_dialect(registry.profile());
    assert!(missing.invocation_metadata_context().is_none());
    assert!(
        missing
            .lower("puts retained")
            .source_metadata_input
            .is_none()
    );
    let mut supplied = Lowerer::with_config(registry, config)
        .with_context_registry(std::sync::Arc::clone(&context))
        .with_dialect(registry.profile());
    supplied.set_source_analysis_options(SourceAnalysisOptions::default());
    assert!(
        supplied
            .invocation_metadata_context()
            .unwrap()
            .matches_registry(registry)
    );
    let module = supplied.lower("puts retained");
    assert!(std::sync::Arc::ptr_eq(
        &module
            .source_metadata_input
            .as_ref()
            .unwrap()
            .context_registry(),
        &context
    ));
    let foreign = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    );
    let mut wrong = Lowerer::with_config(registry, config)
        .with_context_registry(std::sync::Arc::clone(&foreign));
    wrong.set_source_analysis_options(SourceAnalysisOptions::default());
    assert!(
        !wrong
            .invocation_metadata_context()
            .unwrap()
            .matches_registry(registry)
    );
    assert!(!wrong.safe_on_uninit("incr", &["value".to_owned()]));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    let actual = Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(older);
    assert!(!actual.safe_on_uninit("incr", &["value".to_owned()]));
    let current = Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(std::sync::Arc::clone(&context));
    assert!(current.safe_on_uninit("incr", &["value".to_owned()]));
}

#[test]
fn reference_only_command_prefix_does_not_create_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = lower_to_ir("trace remove variable v write changed", &reg());
    let stmt = m.top_level.statements.first().expect("trace remove call");
    assert!(
        matches!(stmt, Statement::Call { command, .. } if command == "trace"),
        "matching a prior registration does not execute the prefix: {stmt:?}",
    );
}

// These tests pin the registry-driven hook-ID dispatch of `if`,
// `switch`, `for`, `while`, `catch`, `try` to the expected shape.

#[test]
fn registry_dispatch_if_lowers_to_if() {
    let m = lower_to_ir("if {1} { set q 4 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::If { .. }),
        "expected If via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_switch_lowers_to_switch() {
    let m = lower_to_ir("switch a { a { set q 1 } b { set q 2 } }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Switch { .. }),
        "expected Switch via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_for_lowers_to_for() {
    let m = lower_to_ir("for {set i 0} {$i < 3} {incr i} { set q $i }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::For { .. }),
        "expected For via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_while_lowers_to_while() {
    let m = lower_to_ir("while {0} { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::While { .. }),
        "expected While via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_catch_lowers_to_catch() {
    let m = lower_to_ir("catch { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Catch { .. }),
        "expected Catch via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_try_lowers_to_try() {
    let m = lower_to_ir("try { set q 1 } on error e { set q 2 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Try { .. }),
        "expected Try via registry hook, got {stmt:?}",
    );
}

// `proc`, `namespace eval`, `foreach`, `lmap`, `dict` dispatch
// through registry-driven hook-ID, with shape preconditions
// enforced inside each match arm.

#[test]
fn registry_dispatch_proc_lowers_to_proc_call_and_registers_procedure() {
    let m = lower_to_ir("proc greet {name} { puts hi }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "proc",
            "expected `proc` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
    assert!(
        fixture_id_if_present(&m.procedures, "::greet").is_some(),
        "expected ::greet to be registered in module.procedures, got keys={:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn registry_dispatch_proc_wrong_arity_falls_through_to_default() {
    // `proc` with two args — fails the `args.len() == 3`
    // precondition.  The registry-driven path returns None,
    // falls through to the residual string match's
    // `_ => lower_default(...)` arm, which produces a
    // runtime `Call` rather than registering a procedure.
    let m = lower_to_ir("proc greet onlytwo", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::greet").is_none(),
        "expected ::greet NOT to be registered when arity is wrong",
    );
}

#[test]
fn registry_dispatch_namespace_eval_lowers_to_barrier() {
    // `lower_namespace_eval` emits a `Statement::Barrier`
    // tagged with `reason: "namespace eval"`.
    let m = lower_to_ir("namespace eval ::myns { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Barrier { reason, .. } => assert_eq!(
            reason, "namespace eval",
            "expected `namespace eval` barrier via registry hook, got reason={reason}",
        ),
        other => panic!("expected Barrier via registry hook, got {other:?}"),
    }
}

#[test]
fn registry_dispatch_namespace_non_eval_subcommand_falls_through() {
    // `namespace import` has no `lowering_hook` on its
    // subcommand spec, so `resolve_call` returns hook=None.
    // The registry-driven dispatcher returns None and the
    // residual string match's `_ => lower_default(...)`
    // arm produces a generic `Call`.
    let m = lower_to_ir("namespace import ::foo::*", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_foreach_lowers_to_foreach() {
    let m = lower_to_ir("foreach v {1 2 3} { set q $v }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_lmap_lowers_to_foreach() {
    // `lmap` shares `lower_foreach(... is_lmap=true)` with
    // `foreach`; both produce `Statement::Foreach`.
    let m = lower_to_ir("lmap v {1 2 3} { expr {$v + 1} }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach (is_lmap=true) via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_dict_lowers_via_lower_dict() {
    // `dict set d k v` exercises the `lower_dict`
    // subcommand-dispatch path.  Pin the registry-driven
    // route by checking the canonical command name on the
    // emitted Call.
    let m = lower_to_ir("dict set d k v", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "dict",
            "expected `dict` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
}

#[test]
fn registry_dispatch_dict_empty_args_falls_through_to_default() {
    // Bare `dict` — fails the `!args.is_empty()`
    // precondition, falls through to `lower_default`.
    let m = lower_to_ir("dict", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Call { .. }),
        "expected Call via lower_default fallback, got {stmt:?}",
    );
}

// `when` and `foreachLine` dispatch through registry-driven
// hook-ID.  `when` is dialect-gated (iRules), so its test
// registry loads the dialect explicitly.  `foreachLine` is
// a Tcl 9.0+ command always registered in `build_default()`.

#[test]
fn registry_dispatch_when_lowers_via_lower_when_with_irules_loaded() {
    // The iRules `when` spec carries `LoweringHookId::When`
    // and only enters the registry after
    // `load_dialect(IRULES)`.  Production callers (the LSP
    // server) always pair `build_default()`
    // with the active dialect; this test mirrors that.
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let m = lower_to_ir_with_config(
        "when HTTP_REQUEST { set q 1 }",
        registry,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "when",
            "expected `when` call via registry hook, got command={command}",
        ),
        other => panic!("expected Call via registry hook, got {other:?}"),
    }
    assert!(
        fixture_id_if_present(&m.procedures, "::when::HTTP_REQUEST").is_some(),
        "expected ::when::HTTP_REQUEST procedure to be registered, got keys={:?}",
        m.procedures.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn registry_dispatch_when_without_irules_loaded_falls_through() {
    // Without `load_irules()`, the registry has no `when`
    // spec, so `resolve_call` returns `None` and
    // `try_dispatch_structured_hook` falls through to
    // `lower_default`.  This is a misconfiguration on the
    // caller's side (the source uses iRules but the
    // registry doesn't know about it); the lowerer emits a
    // generic `Call` rather than silently treating it as
    // an event handler.  Pinning this behaviour catches
    // future drift if `build_default()` ever folds in
    // iRules.
    let m = lower_to_ir("when HTTP_REQUEST { set q 1 }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "when",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
    assert!(
        !m.procedures
            .values()
            .any(|body| body.qualified_name.starts_with("::when::")),
        "expected NO ::when::* procedure registered without iRules loaded",
    );
}

#[test]
fn registry_dispatch_foreach_line_lowers_via_lower_foreach_line() {
    // `foreachLine` is a Tcl 9.0+ command (TIP 670) and
    // `build_default()` registers it via the
    // `tcl::foreachline` spec.  No dialect-load dance is
    // needed.  The body is a brace-string literal here, so
    // the dedicated lowerer relaxes to a typed `Foreach`
    // (not a barrier).
    let m = lower_to_ir("foreachLine line readme.txt { set q $line }", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    assert!(
        matches!(stmt, Statement::Foreach { .. }),
        "expected Foreach via registry hook, got {stmt:?}",
    );
}

#[test]
fn registry_dispatch_foreach_line_wrong_arity_falls_through_to_default() {
    // `foreachLine` with two args — fails the
    // `args.len() == 3` precondition, falls through to
    // `lower_default`.
    let m = lower_to_ir("foreachLine onlytwo args", &reg());
    let stmt = m.top_level.statements.first().expect("at least one stmt");
    match stmt {
        Statement::Call { command, .. } => assert_eq!(
            command, "foreachLine",
            "expected generic Call via lower_default, got command={command}",
        ),
        other => panic!("expected Call via lower_default, got {other:?}"),
    }
}

/// `if {1} { if {1} { ... } }`, `depth` levels deep.
fn nested_if_source(depth: u32) -> String {
    let mut source = String::new();
    for _ in 0..depth {
        source.push_str("if {1} {\n");
    }
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source
}

/// Depth-first search for a [`Statement::Barrier`] whose `reason`
/// contains `needle`, anywhere in `script`'s tree (into `If` clauses and
/// `else` bodies — the only nesting `nested_if_source` produces).
fn contains_barrier_reason(script: &Script, needle: &str) -> bool {
    script.statements.iter().any(|stmt| match stmt {
        Statement::Barrier { reason, .. } => reason.contains(needle),
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .any(|c| contains_barrier_reason(&c.body, needle))
                || else_body
                    .as_ref()
                    .is_some_and(|e| contains_barrier_reason(e, needle))
        }
        _ => false,
    })
}

/// `lower_script` / `lower_body` recurse one Rust frame group per `if`
/// nesting level, so they need the same depth cap as `analyse_body`
/// (`tcl_compiler::analyser::commands::MAX_BODY_DEPTH`) and
/// `cfg_builder::lower_script` (`MAX_LOWER_DEPTH`). A document whose IR
/// lowering runs unguarded defeats the CFG builder's own guard
/// downstream — the crash happens one stage earlier. Lowering 2000
/// levels must neither hang nor overflow the stack, and must record the
/// cap trip as a `Statement::Barrier` (unknown effects) rather than
/// silently truncating to an empty, falsely-dead-looking script.
#[test]
fn deeply_nested_if_past_max_lower_depth_barriers_not_crashes() {
    let source = nested_if_source(2000);
    // `cargo test` runs each test on a small default-stack thread — see
    // the identical helper's doc comment in `analyser::commands::tests`.
    let m = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || lower_to_ir(&source, &reg()))
        .expect("spawn big-stack test thread")
        .join()
        .expect("lower_to_ir on big-stack thread panicked");
    assert!(
        contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
        "expected a depth-cap Barrier somewhere in the lowered tree"
    );
}

/// Depth of the deepest chain of nested `If` statements in `script`.
fn if_chain_depth(script: &Script) -> u32 {
    script
        .statements
        .iter()
        .map(|stmt| match stmt {
            Statement::If { clauses, .. } => {
                1 + clauses.first().map_or(0, |c| if_chain_depth(&c.body))
            }
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn shallow_nested_if_lowers_with_no_barrier() {
    // False-positive guard: ordinary, hand-written nesting must lower
    // as real `If` statements all the way down, never barriered.
    let source = nested_if_source(10);
    let m = lower_to_ir(&source, &reg());
    assert!(
        !contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
        "shallow nesting must not trip the depth cap"
    );
    // 10 real nested `If`s, not a single opaque barrier standing in for
    // all of them.
    assert_eq!(if_chain_depth(&m.top_level), 10);
}

/// Regression (JimTcl/iRules dialect drift): every re-lex of a *body*
/// this lowering performs takes its grammar from the document's own
/// [`tcl_lexer::LexerConfig`], not the Tcl 9.x default.
///
/// `body_has_dynamic_barrier` re-segments a braced body to find a nested
/// dynamic `eval`/`uplevel`.  Under `f5-irules` the lexer treats `}{` as a
/// word separator, so `foo {a}{eval $x}` is **three** words and its third
/// is a braced script containing a dynamic `eval`; under the default
/// grammar the two braces weld into one compound word whose text is
/// `aeval $x`, whose head `aeval` is nobody's barrier.  Before the config
/// was threaded, the iRules document was re-segmented under Tcl rules and
/// the barrier was missed — the outer `eval` relaxed to inline IR that
/// runs a still-dynamic inner `eval`.
#[test]
fn body_relex_follows_the_document_dialect_not_the_default() {
    let irules = tcl_dialect::DialectProfile::irules();
    let irules_config = tcl_lexer::LexerConfig::from_grammar(irules.grammar);
    let registry = tcl_registry::model::ingress::static_context_for_profile(irules).commands();

    let body = "foo {a}{eval $x}";
    assert!(
        !body_has_dynamic_barrier(body, registry, tcl_lexer::LexerConfig::default()),
        "under the default grammar `}}{{` welds one word, so no nested barrier is visible"
    );
    assert!(
        body_has_dynamic_barrier(body, registry, irules_config),
        "under f5-irules `}}{{` separates words, exposing the nested dynamic `eval`"
    );

    // End to end: the same fact decides whether the outer `eval` relaxes
    // to an inline `Block` or stays a runtime `Barrier`.
    let source = "eval {foo {a}{eval $x}}";
    let irules_module = lower_to_ir_with_config(source, registry, irules_config);
    assert_eq!(irules_module.top_level.statements.len(), 1);
    assert_runtime_opaque_call(&irules_module.top_level.statements[0], registry, "eval");

    // Under the default grammar the weld makes `{a}{eval $x}` a *single*
    // word, and that word is not valid Tcl — C answers `extra characters
    // after close-brace` for this very body, on 8.6 and 9.0 alike. So the
    // relaxation is declined here too, by the parse gate rather than the
    // barrier gate (#1829): inlining would lower the malformed text with
    // the lenient lowering and give it a meaning C never gives it.
    let tcl_module = lower_to_ir_with_config(source, registry, tcl_lexer::LexerConfig::default());
    assert!(
        matches!(
            tcl_module.top_level.statements.as_slice(),
            [Statement::Barrier { .. }]
        ),
        "a body that does not parse keeps the runtime barrier: {:?}",
        tcl_module.top_level.statements
    );

    // Positive control: the relaxation is still alive. A body with no
    // nested barrier that *does* parse must still reach the inline Block,
    // so neither assertion above can pass merely because relaxation
    // stopped happening at all.
    let relaxable = lower_to_ir_with_config(
        "eval {foo a b}",
        registry,
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        matches!(
            relaxable.top_level.statements.as_slice(),
            [Statement::Block { .. }]
        ),
        "a parsing, barrier-free body still relaxes to a Block: {:?}",
        relaxable.top_level.statements
    );
}

/// `JimTcl`'s `$(…)` lowers as the expression it is, end to end from the
/// lexer: `set x $($a*2)` becomes the `AssignExpr` a `[expr {$a*2}]`
/// value produces, with the body anchored two bytes past the `$(` so
/// expression diagnostics land on the right source; `return $($a+1)`
/// carries its expression the same way.
#[test]
fn jim_expr_sugar_lowers_as_an_anchored_expression() {
    let jim = tcl_registry::model::resolve_environment("jim").unit_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(jim).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(jim.grammar);
    let module = lower_to_ir_with_config("set x $($a*2)\nreturn $($a+1)\n", registry, config);
    let statements = module.top_level.statements.as_slice();
    match statements {
        [
            Statement::AssignExpr {
                name,
                expr,
                expr_base,
                command_binding,
                ..
            },
            Statement::Return {
                expr: return_expr, ..
            },
        ] => {
            assert_eq!(name, "x");
            let rendered = format!("{expr:?}");
            assert!(rendered.contains('a'), "operand missing from {rendered}");
            // `set x ` is six bytes, `$(` two more: the body starts at 8.
            assert_eq!(*expr_base, Some(8), "the sugar body's absolute anchor");
            assert!(
                command_binding.is_none(),
                "Jim expression syntax must not manufacture an expr command binding"
            );
            assert!(return_expr.is_some(), "return $(…) carries its expression");
        }
        other => panic!("expected AssignExpr + Return; got {other:?}"),
    }
}
