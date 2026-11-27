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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

fn original_cfg_throw_arguments() -> [InvocationWord<'static>; 2] {
    [
        InvocationWord::Literal("CODE"),
        InvocationWord::Literal("MESSAGE"),
    ]
}

fn original_cfg_string_arguments() -> [InvocationWord<'static>; 2] {
    [
        InvocationWord::Literal("length"),
        InvocationWord::Literal("abc"),
    ]
}

fn original_alias_dictionary_binders() -> Vec<DictBinder> {
    vec![
        DictBinder::Key {
            name: "$output".to_owned(),
            value: "ONE".to_owned(),
        },
        DictBinder::Key {
            name: "é(".to_owned(),
            value: "TWO".to_owned(),
        },
    ]
}

#[test]
fn lattice_driver_retains_actual_cfg_availability_and_source_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    use std::sync::Arc;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = LexerConfig {
        expand_syntax: false,
        ..LexerConfig::for_file_grammar(profile.grammar)
    };
    assert!(profile.grammar.expand_syntax);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let older_context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    let older =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, older_context, config);
    let policy = FoldPolicy::for_profile(None, Some(profile));
    let driver = |input| {
        LatticeDriver::new(
            TraceInputs {
                source_metadata_input: input,
                registry: context.commands(),
                traced_variables: &EMPTY_NAMES,
                has_dynamic_variable_trace: false,
                deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                analysis_context: None,
                existence: None,
            },
            None,
            policy,
            &HashSet::new(),
        )
    };
    let throw_arguments = original_cfg_throw_arguments();
    let string_arguments = original_cfg_string_arguments();
    let owner = Owner::for_source_input(Some(&input));
    let entry_driver = driver(Some(&input));
    assert!(!entry_driver.context.grammar.expand_syntax);
    let selected = entry_driver.with_cfg_metadata(&owner);
    assert_eq!(selected.lexer_config, config);
    assert_eq!(selected.source_reference_lexer_config(), Some(config));
    assert!(!selected.context.grammar.expand_syntax);
    assert_eq!(
        selected
            .context
            .profile
            .map(tcl_dialect::DialectProfile::cache_key),
        Some(profile.cache_key())
    );
    assert!(selected.resolve("throw", &throw_arguments).is_some());
    let older_owner = Owner::for_source_input(Some(&older));
    let older_selected = driver(Some(&older)).with_cfg_metadata(&older_owner);
    assert_eq!(older_selected.lexer_config, config);
    assert_eq!(older_selected.source_reference_lexer_config(), Some(config));
    assert!(older_selected.resolve("throw", &throw_arguments).is_none());
    let mismatched = driver(Some(&input)).with_cfg_metadata(&older_owner);
    assert!(mismatched.resolve("string", &string_arguments).is_none());
    assert_eq!(mismatched.source_reference_lexer_config(), None);
    let unavailable_owner = Owner::Unavailable;
    let unavailable = driver(None).with_cfg_metadata(&unavailable_owner);
    assert!(!unavailable.typed_assignment);
    assert_eq!(unavailable.source_reference_lexer_config(), None);
    assert!(unavailable.resolve("string", &string_arguments).is_none());
    let standalone_owner = Owner::Standalone;
    let standalone = driver(None).with_cfg_metadata(&standalone_owner);
    assert!(standalone.resolve("string", &string_arguments).is_some());
    assert!(standalone.source_reference_lexer_config().is_some());
    let availability_owner = Owner::Supplied(Arc::clone(&context));
    assert_eq!(
        driver(None)
            .with_cfg_metadata(&availability_owner)
            .source_reference_lexer_config(),
        None
    );
    let unprofiled = CommandRegistry::build_default();
    let absent = LatticeDriver::new(
        TraceInputs {
            source_metadata_input: None,
            registry: &unprofiled,
            traced_variables: &EMPTY_NAMES,
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
        None,
        policy,
        &HashSet::new(),
    );
    assert_eq!(absent.source_reference_lexer_config(), None);
}

thread_local! {
    /// The request size the next driver on this thread opens, when set.
    pub(super) static REQUEST_WORK: Cell<Option<u64>> = const { Cell::new(None) };
}

/// A recorded word has the value Tcl substitutes, or none: a bare or
/// quoted word has its escapes decoded, and an escaped `$` or `[` is data;
/// a braced word is its content with the line continuation collapsed and
/// nothing else decoded; a word with a live substitution, which an escaped
/// backslash before the `$` does not hide, has no value.
#[test]
fn a_recorded_word_has_the_value_tcl_substitutes() {
    let config = LexerConfig::default();
    let value =
        |text: &str, braced: bool| recorded_word_value(text, braced, &config).map(Cow::into_owned);
    for (text, braced, expected) in [
        (r"a\nb", false, Some("a\nb")),
        (r"a\tb", false, Some("a\tb")),
        (r"a\\b", false, Some(r"a\b")),
        (r"a\$b", false, Some("a$b")),
        (r"a\[b", false, Some("a[b")),
        (r"a\b", true, Some(r"a\b")),
        ("a\\\nb", true, Some("a b")),
        ("${x}", true, Some("${x}")),
        ("a${x}b", false, None),
        ("a[b]", false, None),
        (r"\\$x", false, None),
    ] {
        assert_eq!(
            value(text, braced).as_deref(),
            expected,
            "{text:?} braced: {braced}"
        );
    }
}

/// A run whose evaluations spend its request declines the rest with
/// `Budget(Request)`, published as `Overdefined`: under the default
/// request every `string length` of the procedure folds; under one too
/// small for them, a decline records the request as its reason, and
/// the join keeps every decline, so no definition the request could
/// not pay for publishes a value.
#[test]
fn an_exhausted_request_declines_the_rest() {
    let body: String = (0..12).fold(String::new(), |mut body, i| {
        use std::fmt::Write as _;
        let _ = writeln!(body, "    set a{i} [string length abc{i}]");
        body
    });
    let source = format!("proc p {{}} {{\n{body}}}\n");
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let build = || {
        crate::compilation_unit::CompilationUnit::build_for_dialect(
            &source, registry, false, "tcl9.0",
        )
    };
    let value = |unit: &crate::compilation_unit::CompilationUnit, i: usize| {
        let function = fixture_id_if_present(&unit.procedures, "::p")
            .and_then(|id| unit.procedures.get(&id))
            .expect("the procedure");
        let symbol = function
            .ssa
            .var_symbol(&format!("a{i}"))
            .expect("the variable");
        function.sccp.values.get(&(symbol, 1)).cloned()
    };
    let answers = |unit: &crate::compilation_unit::CompilationUnit| -> Vec<String> {
        fixture_id_if_present(&unit.procedures, "::p")
            .and_then(|id| unit.procedures.get(&id))
            .expect("the procedure")
            .sccp
            .explanations
            .iter()
            .filter(|explanation| explanation.command == "string")
            .map(|explanation| explanation.answer.clone())
            .collect()
    };

    let unit = build();
    for i in 0..12 {
        let expected = if i < 10 { 4 } else { 5 };
        assert_eq!(
            value(&unit, i),
            Some(LatticeValue::Const(ConstValue::int(expected))),
            "a{i} folds under the default request"
        );
    }
    assert!(answers(&unit).iter().all(|answer| answer == "evaluated"));

    REQUEST_WORK.with(|work| work.set(Some(500)));
    let unit = build();
    REQUEST_WORK.with(|work| work.set(None));
    let answers = answers(&unit);
    assert_eq!(answers.len(), 12, "{answers:?}");
    assert!(
        answers
            .iter()
            .any(|answer| answer == "declined: budget: Request"),
        "{answers:?}"
    );
    for (i, answer) in answers.iter().enumerate() {
        if answer.starts_with("declined") {
            assert_eq!(
                value(&unit, i),
                Some(LatticeValue::Overdefined),
                "a{i}: {answer}"
            );
        }
    }
    assert_eq!(
        value(&unit, 11),
        Some(LatticeValue::Overdefined),
        "the last statement is past what the request pays for"
    );
}

/// A whole-array writer's evaluated element writes reach the lattice:
/// `array set arr {k1 v1 k2 v2}` names `arr(k1)` and `arr(k2)`, which the
/// SSA fans out as may-writes of the array's known elements, and each
/// takes the written value rather than its join with the prior version.
/// An element the pairs do not name stays a may-write and widens, and a
/// dynamic-key write (`set arr($i) x`) still joins every element it may
/// have hit with its prior.
#[test]
fn an_array_set_writes_the_elements_it_names() {
    let source = "proc p {i} {\n    set arr(k3) keep\n    array set arr {k1 v1 k2 v2}\n    \
                      puts \"$arr(k1) $arr(k2) $arr(k3)\"\n    set arr($i) x\n    puts $arr(k1)\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    let value = |name: &str, version: u32| {
        let symbol = function.ssa.var_symbol(name).expect("the variable");
        function.sccp.values.get(&(symbol, version)).cloned()
    };
    let text = |value: &str| Some(LatticeValue::Const(ConstValue::String(value.into())));
    assert_eq!(value("arr(k1)", 1), text("v1"));
    assert_eq!(value("arr(k2)", 1), text("v2"));
    assert_eq!(value("arr(k3)", 1), text("keep"));
    assert_eq!(
        value("arr(k3)", 2),
        Some(LatticeValue::Overdefined),
        "an element the pairs do not name is a may-write"
    );
    assert_ne!(
        value("arr(k1)", 2),
        text("v1"),
        "a dynamic key may have hit the element"
    );
}

/// A destructuring route resolves its targets' roles over the values the
/// lattice proves: a substituted subject no
/// longer hides them, so `regexp` writes its match variable, `lassign`
/// both of its targets, and a no-match preserves its target — tclsh 8.5
/// to 9.1 print `aa`, `1 2` and `before`.
#[test]
fn a_proven_subject_resolves_the_targets_roles() {
    let source = "proc p {} {\n    set s aab\n    regexp {(a+)b} $s -> g\n    \
                      set l {1 2}\n    lassign $l a b\n    set t zz\n    set m before\n    \
                      regexp {(x)} $t -> m\n    puts \"$g $a $b $m\"\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    let last = |name: &str| {
        let symbol = function.ssa.var_symbol(name).expect("the variable");
        function
            .sccp
            .values
            .iter()
            .filter(|((sym, _), _)| *sym == symbol)
            .max_by_key(|((_, version), _)| *version)
            .map(|(_, value)| value.clone())
    };
    let text = |value: &str| Some(LatticeValue::Const(ConstValue::String(value.into())));
    assert_eq!(last("g"), text("aa"));
    assert_eq!(last("a"), Some(LatticeValue::Const(ConstValue::int(1))));
    assert_eq!(last("b"), Some(LatticeValue::Const(ConstValue::int(2))));
    assert_eq!(last("m"), text("before"), "the no-match preserves `m`");
}

/// `proven_word_value` reads the lattice at the statement: a
/// word reading a variable a `set` gave a literal is that literal, one
/// reading a route's result carries the folded type the route states, a
/// quoted word is its runs and reads concatenated and a literal word its
/// text; the `$f` token is never the text `$f`. A word the lattice cannot
/// pin is `None`: a command substitution, and the same `$f` after a
/// branch redefines it.
#[test]
fn proven_word_value_reads_the_lattice_at_the_statement() {
    let source = "proc p {c} {\n    set f %d\n    set n [string length abc]\n    \
                      format $f $n \"x$f\" 7 [string length ab]\n    \
                      if {$c} { set f %s }\n    format $f 1\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = LexerConfig::for_profile(registry.profile());
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    // The span of `word` in the first place `call` is written.
    let word_at = |call: &str, word: &str| {
        let start = source.find(call).expect("the call") + call.find(word).expect("the word");
        function
            .word_at(Span::new(
                u32::try_from(start).expect("offset"),
                u32::try_from(start + word.len()).expect("offset"),
            ))
            .expect("a word of a reached call")
    };
    let (first, word) = word_at("format $f $n", "$f");
    assert_eq!(word, 1);
    let value = |statement: StatementId, word: usize| {
        proven_word_value(function, statement, word, config)
            .map(|(value, folded)| (String::from_utf8(value.bytes).expect("text"), folded))
    };
    assert_eq!(value(first, 1), Some(("%d".to_owned(), None)));
    let (length, folded) = value(first, 2).expect("the route's result");
    assert_eq!(length, "3");
    assert_eq!(
        folded.and_then(|folded| folded.intrep),
        Some(tcl_registry::TclType::Int)
    );
    assert_eq!(value(first, 3), Some(("x%d".to_owned(), None)));
    assert_eq!(value(first, 4), Some(("7".to_owned(), None)));
    assert_eq!(value(first, 5), None, "a command substitution");
    assert_eq!(value(first, 0), None, "the command word");
    let (second, word) = word_at("format $f 1", "$f");
    assert_eq!(word, 1);
    assert_ne!(second, first);
    assert_eq!(value(second, 1), None, "a branch redefines `f`");
}

/// A loop header binds each binder of its plan the elements it is
/// assigned: `foreach {a b} {1 10 2 20 3} {…}` gives `a` the
/// set `{1 2 3}` and `b` the set `{10 20 ""}`, the empty string where the
/// last iteration runs past the list's end; a repeated binder holds its
/// last position's element (`foreach {c c} {x y z w}` gives `c` `{y w}`).
#[test]
fn a_loop_header_binds_each_binder_its_elements() {
    let source = "proc p {} {\n    foreach {a b} {1 10 2 20 3} {\n        puts $a$b\n    }\n    \
                      foreach {c c} {x y z w} {\n        puts $c\n    }\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    let set_of = |texts: &[&str]| {
        LatticeValue::constset(
            texts
                .iter()
                .map(|text| crate::sccp::parse_literal_value(text))
                .collect(),
        )
    };
    let holds = |name: &str, expected: &LatticeValue| {
        let symbol = function.ssa.var_symbol(name).expect("the variable");
        function
            .sccp
            .values
            .iter()
            .any(|((sym, _), value)| *sym == symbol && value == expected)
    };
    assert!(
        holds("a", &set_of(&["1", "2", "3"])),
        "{:?}",
        function.sccp.values
    );
    assert!(
        holds("b", &set_of(&["10", "20", ""])),
        "{:?}",
        function.sccp.values
    );
    assert!(
        holds("c", &set_of(&["y", "w"])),
        "{:?}",
        function.sccp.values
    );
}

#[test]
fn original_dictionary_body_plan_keeps_aliases_literal_names_and_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original lexical binder plan only; the supplied dictionary is a
    // separate premise, with no successful body entry or store claim.
    use std::sync::Arc;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let registry = context.commands();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let source = "interp alias {} unpack {} dict with {$dictionary}; unpack {}";
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    let module = lowerer.lower(source);
    let statement = module
        .top_level
        .statements
        .last()
        .expect("original alias call");
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .expect("original source argv");
    assert_eq!(
        dict_body_operand_with_metadata_context(registry, metadata, tokens),
        Some((1, "$dictionary".to_owned()))
    );
    assert_eq!(
        dict_body_with_metadata_context(registry, metadata, tokens, "$output ONE é( TWO"),
        Some(original_alias_dictionary_binders())
    );
    let older_context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let older =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, older_context, config);
    let older_metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &older)
            .unwrap();
    assert!(dict_body_operand_with_metadata_context(registry, older_metadata, tokens).is_none());
    assert!(
        dict_body_with_metadata_context(registry, (&*context).into(), tokens, "$output ONE")
            .is_none(),
        "availability-only input cannot invent source context"
    );
    let foreign_context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let foreign =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, foreign_context, config);
    assert!(
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &foreign
        )
        .is_none()
    );
    let stale = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        LexerConfig {
            expand_syntax: !config.expand_syntax,
            ..config
        },
    );
    let stale_metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &stale)
            .unwrap();
    assert!(dict_body_operand_with_metadata_context(registry, stale_metadata, tokens).is_none());
    let mut replaced = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    let changed = replaced.lower("proc dict args {return CUSTOM}; dict with {$dictionary} {}");
    let changed_call = changed
        .top_level
        .statements
        .last()
        .expect("original replacement call");
    let changed_tokens = changed
        .top_level
        .retained_source_tokens_for_statement(changed_call)
        .expect("original replacement argv");
    assert!(dict_body_operand_with_metadata_context(registry, metadata, changed_tokens).is_none());
}

/// A dictionary body is found by its plan whatever its dictionary holds:
/// the probe's dictionary holds the key path the plan reads,
/// so `dict with d a b {…}` over a dictionary the analysis does not know
/// is still one, under either spelling. Over a known dictionary the
/// declared keys are read at the key path, which a dictionary lacking
/// it fails as the command does, and a `dict update` variable is bound
/// only when the dictionary holds its key.
#[test]
fn a_dictionary_body_is_found_by_its_plan() {
    let registry = CommandRegistry::build_default();
    let args =
        |words: &[&str]| -> Vec<String> { words.iter().map(|&word| word.to_owned()).collect() };
    let operand = |head: &str, words: &[&str]| dict_body_operand(&registry, head, &args(words));
    assert_eq!(operand("dict", &["with", "d", "{}"]), Some(1));
    assert_eq!(operand("dict", &["with", "d", "a", "b", "{}"]), Some(1));
    assert_eq!(operand("::tcl::dict::with", &["d", "a", "{}"]), Some(0));
    assert_eq!(operand("dict", &["update", "d", "k", "v", "{}"]), Some(1));
    assert_eq!(operand("dict", &["set", "d", "k", "v"]), None);
    assert_eq!(operand("foreach", &["x", "{1 2}", "{}"]), None);
    let key = |name: &str, value: &str| DictBinder::Key {
        name: name.to_owned(),
        value: value.to_owned(),
    };
    let body = |head: &str, words: &[&str], dictionary: &str| {
        dict_body(&registry, head, &args(words), dictionary)
    };
    assert_eq!(
        body("dict", &["with", "d", "{}"], "x 1 y 2 x 3"),
        Some(vec![key("x", "3"), key("y", "2")])
    );
    assert_eq!(
        body("dict", &["with", "d", "a", "{}"], "a {x 1} b {y 2}"),
        Some(vec![key("x", "1")])
    );
    assert_eq!(body("dict", &["with", "d", "z", "{}"], "a {x 1}"), None);
    assert_eq!(
        body(
            "dict",
            &["update", "d", "k", "v", "j", "w", "$m", "u", "{}"],
            "k 1 $m 2"
        ),
        Some(vec![
            DictBinder::Variable {
                variable: 3,
                bound: true
            },
            DictBinder::Variable {
                variable: 5,
                bound: false
            },
            DictBinder::Variable {
                variable: 7,
                bound: false
            },
        ])
    );
}

/// The driver records each executable `subst` call's template-word
/// plan over the settled lattice: `set opt -novariables; subst
/// $opt {hello $name}` reads the proven switch, so the template's
/// `$name` is no read (tclsh 8.4 to 9.1 print `hello $name`); each record
/// carries its template word's token span, the plan's own spans offsets
/// into it; a call in a block the solver never reaches is not recorded.
#[test]
fn a_subst_call_records_its_template_plan() {
    use tcl_registry::substitution::SubstitutionKinds;
    let source = "proc p {} {\n    set opt -novariables\n    subst $opt {hello $name}\n    \
                      subst -nocommands {a$b}\n    if {0} { subst {dead $x} }\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    let records = &function.sccp.template_plans;
    let word = |text: &str| {
        let start = source.find(text).expect("the word");
        let at = |offset: usize| u32::try_from(offset).expect("a short source");
        Span::new(at(start), at(start + text.len()))
    };
    assert_eq!(
        records.iter().map(|record| record.span).collect::<Vec<_>>(),
        // A braced word's token runs from its `{` to its content's end.
        [word("{hello $name"), word("{a$b")],
        "{records:?}"
    );
    assert_eq!(
        records[0].plan.kinds,
        SubstitutionKinds {
            backslashes: true,
            commands: true,
            variables: false,
        }
    );
    assert!(records[0].plan.reads.is_empty(), "{records:?}");
    assert!(records[0].plan.braced && !records[0].plan.dynamic);
    let reads: Vec<(&str, Span)> = records[1]
        .plan
        .reads
        .iter()
        .map(|read| (read.name.as_str(), read.span))
        .collect();
    assert_eq!(reads, [("b", Span::new(2, 4))]);
}

/// The selection records of `::p` in `source` under `dialect`.
fn selections_of(source: &str, dialect: &str) -> Vec<crate::sccp::SelectionRecord> {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure")
        .sccp
        .selections
        .clone()
}

/// The driver records each executable opaque case-list statement's
/// selection over the settled lattice: the record's arm indices
/// count the command's pattern and body pairs against the statement's
/// arms, whose pattern spans it keeps in order, so the index one past
/// them names the final `default` the statement keeps as its default
/// body — `abc` selects `a*`, a literal `zzz` the default, and a
/// finite subject one arm per member. A subject with no proven value,
/// and a flattened exact switch, which is no statement at all, record
/// nothing; `case`'s statement records through the command its binding
/// site names. Each answer is tclsh 8.6.18's.
#[test]
fn an_opaque_switch_records_its_selection() {
    let source = "proc p {x} {\n\
                      set s abc\n\
                      switch -glob -- $s {a* {set r A} default {set r D}}\n\
                      switch -glob -- zzz {a* {set r A} default {set r D}}\n\
                      switch -glob -- $x {a* {set r A} default {set r D}}\n\
                      if {$x} {set t b} else {set t a}\n\
                      switch -glob -- $t {a {set r A} b {set r B} default {set r D}}\n\
                      switch -- $s {abc {set r A} default {set r D}}\n\
                      case abc in a* {set r A} default {set r D}\n\
                      }\n";
    let records = selections_of(source, "tcl8.6");
    let at = |nth: usize, text: &str| {
        let start = source.match_indices(text).nth(nth).expect("the word").0;
        let at = |offset: usize| u32::try_from(offset).expect("a short source");
        Span::new(at(start), at(start + text.len()))
    };
    assert_eq!(records.len(), 4, "{records:#?}");
    // `abc` selects the first arm, whose body runs.
    assert_eq!(records[0].fact.selected, [Some(0)]);
    assert_eq!(records[0].fact.bodies, [Some(0)]);
    assert_eq!(records[0].arm_pattern_spans, [at(0, "a*")]);
    assert!(!records[0].is_default(0));
    // A literal subject no pattern matches selects the default: the
    // index one past the kept arms.
    assert_eq!(records[1].fact.selected, [Some(1)]);
    assert_eq!(records[1].arm_pattern_spans, [at(1, "a*")]);
    assert!(records[1].is_default(1));
    // A finite subject records one arm per member.
    let mut members = records[2].fact.selected.clone();
    members.sort_unstable();
    assert_eq!(members, [Some(0), Some(1)]);
    assert_eq!(
        records[2].arm_pattern_spans,
        [at(0, "a {set r A} b"), at(0, "b {set r B}")]
            .map(|span| Span::new(span.start(), span.start() + 1))
    );
    // `case` records through its binding site; its statement spans the
    // whole command.
    assert_eq!(records[3].fact.selected, [Some(0)]);
    assert_eq!(records[3].span.start(), at(0, "case abc").start());
    assert!(
        records
            .iter()
            .all(|record| record.fact.writes.iter().all(Vec::is_empty))
    );
}

/// A delimited fall-through body reads two ways on 9.1b0, and
/// the statement carries each word's delimiters to the transfer: `a`
/// falls through a quoted or braced `-` into `b`'s body under 8.6 and
/// 9.0, and no selection is recorded under a profile that may be 9.1;
/// a bare `-` records under every one.
#[test]
fn a_delimited_fallthrough_body_records_no_selection_under_91() {
    for (body, under_91) in [("\"-\"", false), ("{-}", false), ("-", true)] {
        let source =
            format!("proc p {{}} {{\nset s a\nswitch -glob -- $s a {body} b {{set r B}}\n}}\n");
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
            let records = selections_of(&source, dialect);
            let recorded = matches!(dialect, "tcl8.6" | "tcl9.0") || under_91;
            assert_eq!(records.len(), usize::from(recorded), "{dialect}: {body}");
            if recorded {
                assert_eq!(records[0].fact.selected, [Some(0)], "{dialect}: {body}");
                assert_eq!(records[0].fact.bodies, [Some(1)], "{dialect}: {body}");
            }
        }
    }
}

/// Each arm no member of the subject runs the body of is a `Selected`
/// branch fact — no target, `false`, its pattern's text and span, and
/// the block holding the statement — beside the record, and no
/// reachability is applied for it: every block stays executable and no
/// decided branch appears. The final `default` has no pattern, so it is
/// never one; an arm a `-` body passes through to a running body is not.
#[test]
fn an_unreached_arm_is_a_selected_branch_fact() {
    use crate::sccp::BranchFactKind;
    let source = "proc p {} {\n\
                      set s abc\n\
                      switch -glob -- $s {a* {set r A} b* {set r B} default {set r D}}\n\
                      switch -glob -- $s {a* - z* {set r A} y* {set r B}}\n\
                      }\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    let facts = &function.sccp.constant_branches;
    assert!(
        facts
            .iter()
            .all(|fact| fact.kind == BranchFactKind::Selected)
    );
    let patterns: Vec<&str> = facts.iter().map(|fact| fact.condition.as_str()).collect();
    // `abc` selects `a*` (and, through `-`, `z*`'s body) in the second
    // statement: only `y*` is never run there.
    assert_eq!(patterns, ["b*", "y*"], "{facts:#?}");
    let start = source.find("b* {").expect("the arm");
    let at = |offset: usize| u32::try_from(offset).expect("a short source");
    assert_eq!(facts[0].span, Some(Span::new(at(start), at(start + 2))));
    for fact in facts {
        assert!(!fact.value && fact.taken_target.is_empty() && fact.not_taken_target.is_empty());
        assert!(
            function
                .cfg
                .block_by_name(&fact.block)
                .is_some_and(|block| block
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, Statement::Switch { .. }))),
            "the block holding the statement: {fact:?}"
        );
    }
    assert_eq!(
        function.sccp.executable_blocks.len(),
        function.cfg.blocks.len()
    );
}

/// A module that defines its own `switch` is not trusted to select as
/// the builtin does: its opaque statement stays, and records nothing.
#[test]
fn a_redefined_switch_records_no_selection() {
    let source = "proc switch {args} {return x}\n\
                      proc p {} {\nswitch -glob -- abc {a* {set r A} default {set r D}}\n}\n";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the procedure");
    assert!(
        function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(statement.statement, Statement::Switch { .. })),
        "the opaque statement"
    );
    assert_eq!(
        function.sccp.selections,
        [] as [crate::sccp::SelectionRecord; 0]
    );
}

/// The selection the call `head words…` makes under `dialect`.
fn call_selection(
    dialect: &str,
    head: &str,
    words: &[(&str, WordForm)],
    subject: Option<&str>,
) -> Option<LiteralSelection> {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    literal_selection(registry, head, words, subject)
}

/// A consumer holding a call's words gets the command's own selection —
/// mode, case folding, the final `default`, fall-through — from the
/// registry, and the subject a caller proves for a word the call spells
/// as a variable. Each answer is tclsh 8.6.18's.
#[test]
fn a_literal_call_selects_through_the_registry() {
    use WordForm::{Bare, Braced};
    let glob = [
        ("-glob", Bare),
        ("--", Bare),
        ("abc", Bare),
        ("a*", Bare),
        ("A", Braced),
        ("default", Bare),
        ("D", Braced),
    ];
    let chosen = call_selection("tcl8.6", "switch", &glob, None).expect("a selection");
    assert_eq!(chosen.fact.selected, [Some(0)]);
    assert_eq!(chosen.fact.bodies, [Some(0)]);
    assert_eq!((chosen.subject, chosen.clauses), (2, 2));
    // The subject the call spells as `$x`, given the value the caller
    // proves, selects the final default.
    let mut variable = glob;
    variable[2] = ("$x", WordForm::Unproven);
    assert!(call_selection("tcl8.6", "switch", &variable, None).is_none());
    let chosen = call_selection("tcl8.6", "switch", &variable, Some("zzz")).expect("selected");
    assert_eq!(chosen.fact.selected, [Some(1)]);
    assert_eq!(chosen.fact.bodies, [Some(1)]);
    // The regexp mode runs the engine; `-nocase` needs a release that has it.
    let regexp = [
        ("-regexp", Bare),
        ("--", Bare),
        ("abc", Bare),
        ("^a.c$", Bare),
        ("A", Braced),
    ];
    let chosen = call_selection("tcl8.6", "switch", &regexp, None).expect("a selection");
    assert_eq!(chosen.fact.selected, [Some(0)]);
    let nocase = [
        ("-nocase", Bare),
        ("--", Bare),
        ("ABC", Bare),
        ("abc", Bare),
        ("A", Braced),
    ];
    assert!(call_selection("tcl8.6", "switch", &nocase, None).is_some());
    assert!(call_selection("tcl", "switch", &nocase, None).is_none());
    // A pattern the caller does not prove declines the whole selection.
    let mut unproven = glob;
    unproven[3] = ("$p", WordForm::Unproven);
    assert!(call_selection("tcl8.6", "switch", &unproven, None).is_none());
}

/// A `-` body reads two ways on 9.1b0 when it is delimited: a bare one
/// falls through everywhere, a quoted one only where the profile rules
/// 9.1 out.
#[test]
fn a_delimited_fallthrough_word_declines_only_where_it_may_be_91() {
    use WordForm::{Bare, Braced, Quoted};
    for (form, under_91) in [(Bare, true), (Quoted, false), (Braced, false)] {
        let words = [
            ("--", Bare),
            ("a", Bare),
            ("a", Bare),
            ("-", form),
            ("b", Bare),
            ("B", Braced),
        ];
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
            let chosen = call_selection(dialect, "switch", &words, None);
            let expected = matches!(dialect, "tcl8.6" | "tcl9.0") || under_91;
            assert_eq!(chosen.is_some(), expected, "{dialect}: {form:?}");
            if let Some(chosen) = chosen {
                assert_eq!(chosen.fact.bodies, [Some(1)], "{dialect}: {form:?}");
            }
        }
    }
}

/// `case` reads its own contract through the same query: glob patterns
/// and the `in` word skipped, from the releases that have it.
#[test]
fn a_literal_case_call_selects_its_glob_arm() {
    use WordForm::{Bare, Braced};
    let words = [
        ("abc", Bare),
        ("in", Bare),
        ("a*", Bare),
        ("A", Braced),
        ("default", Bare),
        ("D", Braced),
    ];
    for dialect in ["tcl8.4", "tcl8.6"] {
        let chosen = call_selection(dialect, "case", &words, None).expect("a selection");
        assert_eq!(chosen.fact.selected, [Some(0)], "{dialect}");
    }
    assert!(call_selection("tcl9.0", "case", &words, None).is_none());
}

/// The lowered statement's selection is asked over its recorded words,
/// and only where they are the statement's own: a subject an inliner
/// renamed, or words whose delimiters the lowering did not record, make
/// none.
#[test]
fn a_lowered_statement_selects_over_its_recorded_words() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let module = crate::lowering::lower_to_ir(
        "switch -glob -- $mode {a* {set v 1} b {set v 2} default {set v 3}}\n",
        registry,
    );
    let statement = module.top_level.statements[0].clone();
    assert!(matches!(statement, Statement::Switch { .. }));
    let selected = |statement: &Statement, subject: &str| {
        statement_selection(registry, statement, subject).map(|fact| fact.bodies)
    };
    assert_eq!(selected(&statement, "abc"), Some(vec![Some(0)]));
    assert_eq!(selected(&statement, "b"), Some(vec![Some(1)]));
    assert_eq!(selected(&statement, "zzz"), Some(vec![Some(2)]));
    let mut renamed = statement.clone();
    if let Statement::Switch { subject, .. } = &mut renamed {
        *subject = "$other".to_owned();
    }
    assert_eq!(selected(&renamed, "abc"), None);
    let mut unrecorded = statement;
    if let Statement::Switch { raw_arg_quoted, .. } = &mut unrecorded {
        raw_arg_quoted.clear();
    }
    assert_eq!(selected(&unrecorded, "abc"), None);
}

#[test]
fn split_head_basic() {
    assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
    assert_eq!(split_head("  cmd"), ("cmd", None));
    assert_eq!(split_head(""), ("", None));
}

/// An expression's answer names every binding it rests on: the `expr`
/// head, each nested command's head, and each math function (the
/// wrapper command it dispatches to).
#[test]
fn an_expression_answer_names_every_binding_it_rests_on() {
    let registry = CommandRegistry::build_default();
    let mutations = crate::command_binding::ModuleCommandMutations::default();
    let driver = LatticeDriver::detached(
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: crate::sccp::FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        FoldPolicy::default(),
    );
    let node = crate::expr_parser::parse_expr_for_profile("abs([string length abc] - 5)", None);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(&node),
        policy: FoldPolicy::default(),
        nested: NestedPolicy::EffectFreeOnly,
        head: Some(binding_of("expr", "expr")),
    };
    let ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
    let uses: HashMap<Symbol, Version> = HashMap::new();
    let values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let LiftedAnswer::Evaluated(outcomes) =
        driver.evaluate_expression_at(&expression, None, &uses, &values, &ssa)
    else {
        panic!("the expression evaluates");
    };
    let [outcome] = outcomes.as_slice() else {
        panic!("one outcome");
    };
    assert_eq!(
        outcome.result,
        ExactValueOrUnavailable::Exact(ExactValue::int(2))
    );
    let named: Vec<(&str, &str)> = outcome
        .evidence
        .bindings
        .iter()
        .map(|binding| (binding.name.as_str(), binding.identity.as_str()))
        .collect();
    assert_eq!(
        named,
        [
            ("expr", "expr"),
            ("string", "string"),
            ("abs", "::tcl::mathfunc::abs")
        ]
    );
}

/// Each nested write of an outcome as `(place, value)`, in order.
type Written = Vec<(String, String)>;

/// An evaluated answer's one outcome as its result and its nested
/// writes.
fn result_and_writes(answer: &LiftedAnswer) -> (String, Written) {
    let LiftedAnswer::Evaluated(outcomes) = answer else {
        panic!("the expression evaluates: {answer:?}");
    };
    let [outcome] = outcomes.as_slice() else {
        panic!("one outcome: {outcomes:?}");
    };
    let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
        panic!("an exact result: {outcome:?}");
    };
    let writes = outcome
        .nested_writes
        .iter()
        .map(|(place, store)| match store {
            StoreOutcome::Write { value, .. } => (
                place.name.clone(),
                String::from_utf8_lossy(&value.bytes).into_owned(),
            ),
            other => panic!("a write: {other:?}"),
        })
        .collect();
    (String::from_utf8_lossy(&result.bytes).into_owned(), writes)
}

/// Under `LocalWrites` a nested write is applied to the evaluation's
/// state in order, so the next read sees it: over `x` = 1,
/// `$x + [incr x] + $x` reads 1, then 2 (the value is 5), and the state
/// holds the one write. A branch the engine never reaches applies
/// nothing, and the final value of a place written twice is the last.
#[test]
fn local_writes_apply_in_order() {
    let ordered = |text: &str| result_and_writes(&evaluate_over_x(text, NestedPolicy::LocalWrites));
    let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
    let cases: [(&str, &str, Written); 7] = [
        ("$x + [incr x] + $x", "5", vec![write("x", "2")]),
        ("0 && [incr x]", "0", vec![]),
        ("$x + [set x 10] + $x", "21", vec![write("x", "10")]),
        (
            "[incr x] + [incr x]",
            "5",
            vec![write("x", "2"), write("x", "3")],
        ),
        ("$x ? [incr x] : [incr x 10]", "2", vec![write("x", "2")]),
        ("!$x ? [incr x] : [incr x 10]", "11", vec![write("x", "11")]),
        // A quoted operand substitutes its variables and commands in
        // order under the same state.
        ("\"$x [incr x]\" eq {1 2}", "1", vec![write("x", "2")]),
    ];
    for (text, result, writes) in cases {
        assert_eq!(ordered(text), (result.to_owned(), writes), "{text}");
    }
}

/// The same programs under `EffectFreeOnly` decline as stateful where a
/// nested command writes, and a write the state cannot own — a global —
/// declines under `LocalWrites` too.
#[test]
fn a_nested_write_outside_the_state_declines() {
    for text in [
        "$x + [incr x] + $x",
        "$x + [set x 10] + $x",
        "[incr x] + [incr x]",
    ] {
        assert_eq!(
            evaluate_over_x(text, NestedPolicy::EffectFreeOnly),
            LiftedAnswer::Declined(DeclineReason::StatefulNested),
            "{text}"
        );
    }
    // Never reached: no write, so the effect-free policy answers.
    let (result, writes) = result_and_writes(&evaluate_over_x(
        "0 && [incr x]",
        NestedPolicy::EffectFreeOnly,
    ));
    assert_eq!((result.as_str(), writes.len()), ("0", 0));
    for policy in [NestedPolicy::EffectFreeOnly, NestedPolicy::LocalWrites] {
        assert_eq!(
            evaluate_over_x("$x + [incr ::g]", policy),
            LiftedAnswer::Declined(DeclineReason::StatefulNested),
            "{policy:?}"
        );
    }
}

/// `script` run as one command, under `policy`, by a detached driver over
/// a frame where `x` holds 1, `s` holds `a` and `::g` is 5.
fn run_over_frame(script: &str, policy: NestedPolicy) -> ScriptRun {
    let registry = CommandRegistry::build_default();
    let mutations = crate::command_binding::ModuleCommandMutations::default();
    let driver = LatticeDriver::detached(
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: crate::sccp::FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        FoldPolicy::default(),
    );
    let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
    let (x, s, g) = (
        ssa.intern_var("x"),
        ssa.intern_var("s"),
        ssa.intern_var("::g"),
    );
    let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1), (s, 1), (g, 1)]);
    let values: HashMap<ValueKey, LatticeValue> = HashMap::from([
        ((x, 1), LatticeValue::Const(ConstValue::int(1))),
        ((s, 1), LatticeValue::Const(ConstValue::String("a".into()))),
        ((g, 1), LatticeValue::Const(ConstValue::int(5))),
    ]);
    driver
        .run_script(script, (&uses, &values, &ssa), (Vec::new(), policy), None)
        .expect("the script is one command the registry resolves")
}

/// A run's one outcome's result, and every write it places in order: the
/// words' first, then the command's own.
fn run_result_and_writes(run: &ScriptRun) -> (String, Written) {
    let (result, _) = result_and_writes(&run.answer);
    let [placed] = run.writes.as_slice() else {
        panic!("one outcome's writes: {:?}", run.writes);
    };
    let writes = placed
        .iter()
        .map(|(place, store)| match store {
            StoreOutcome::Write { value, .. } => (
                place.name.clone(),
                String::from_utf8_lossy(&value.bytes).into_owned(),
            ),
            other => panic!("a write: {other:?}"),
        })
        .collect();
    (result, writes)
}

/// A command's own substituting words run in order under one state before
/// the command does, whatever route the command takes: what a word wrote
/// is read by the command and is the first of the run's writes. `incr x
/// [incr x]` over `x` = 1 runs the inner `incr` (2), then adds it (4) —
/// reading `x` before the word ran adds it to 1 — and `string length
/// [append s bc]` is the length of the new value, a route that keeps no
/// ordered state of its own, with the append among its writes.
#[test]
fn a_commands_words_run_before_it_and_their_writes_are_the_first_of_its_own() {
    let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
    let cases: [(&str, &str, Written); 6] = [
        (
            "incr x [incr x]",
            "4",
            vec![write("x", "2"), write("x", "4")],
        ),
        ("string length [append s bc]", "3", vec![write("s", "abc")]),
        (
            "incr x \"[incr x]\"",
            "4",
            vec![write("x", "2"), write("x", "4")],
        ),
        (
            "append s [append s b]",
            "abab",
            vec![write("s", "ab"), write("s", "abab")],
        ),
        ("list $x [incr x] $x", "1 2 2", vec![write("x", "2")]),
        ("list [incr x] $x", "2 2", vec![write("x", "2")]),
    ];
    for (script, result, writes) in cases {
        let run = run_over_frame(script, NestedPolicy::LocalWrites);
        assert_eq!(
            run_result_and_writes(&run),
            (result.to_owned(), writes),
            "{script}"
        );
        assert_eq!(
            run_over_frame(script, NestedPolicy::EffectFreeOnly).answer,
            LiftedAnswer::Declined(DeclineReason::StatefulNested),
            "{script}"
        );
    }
}

/// The expression route's quoted operand is the word substituted before
/// the engine parses it, once, under the same state as the commands in it:
/// `expr "$x + [incr x]"` over `x` = 1 is `1 + 2`, however often the route
/// reads the word, and leaves one write. An operand the engine substitutes
/// itself is read after every word has run: `expr {$x} + [incr x]` joins
/// its words into `$x + 2` and reads `x` then, so it is `2 + 2`.
#[test]
fn a_quoted_expr_operand_is_substituted_once_under_the_ordered_state() {
    let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
    for (script, result) in [
        ("expr \"$x + [incr x]\"", "3"),
        ("expr \"[incr x] + 0\"", "2"),
        ("expr \"[incr x]\"", "2"),
        ("expr {$x} + [incr x]", "4"),
    ] {
        let run = run_over_frame(script, NestedPolicy::LocalWrites);
        assert_eq!(
            run_result_and_writes(&run),
            (result.to_owned(), vec![write("x", "2")]),
            "{script}"
        );
    }
    let run = run_over_frame("expr \"$x + [incr x]\"", NestedPolicy::EffectFreeOnly);
    assert_eq!(
        run.answer,
        LiftedAnswer::Declined(DeclineReason::StatefulNested)
    );
}

/// The definitions an embedded call takes from its host's writes: a place
/// no definition belongs to, and a cell written beside the array that
/// holds it, are writes no definition carries, and a place no write names
/// keeps what it held.
#[test]
fn the_definitions_decline_a_write_none_of_them_can_carry() {
    let driver = LatticeDriver::detached(None, FoldPolicy::default());
    let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
    let (x, cell, array) = (
        ssa.intern_var("x"),
        ssa.intern_var("a(1)"),
        ssa.intern_var("a"),
    );
    let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1)]);
    let values: HashMap<ValueKey, LatticeValue> =
        HashMap::from([((x, 1), LatticeValue::Const(ConstValue::int(1)))]);
    let inputs = driver.expression_inputs((&uses, &values, &ssa));
    let defs = vec![
        ("x".to_owned(), (x, 2)),
        ("a(1)".to_owned(), (cell, 2)),
        ("a".to_owned(), (array, 2)),
    ];
    let write = |name: &str, text: &str| {
        (
            place_named(name),
            StoreOutcome::Write {
                target: TargetId(OperandId(0)),
                value: ExactValue::from_literal(text),
            },
        )
    };
    assert!(
        driver
            .embedded_defs(&[write("y", "1")], &defs, &inputs)
            .is_none(),
        "a place no definition belongs to"
    );
    assert!(
        driver
            .embedded_defs(&[write("a(1)", "1"), write("a", "2")], &defs, &inputs)
            .is_none(),
        "a cell and its array"
    );
    let answers = driver
        .embedded_defs(&[write("a(1)", "9")], &defs, &inputs)
        .expect("a write a definition carries");
    assert_eq!(answers[0].value, LatticeValue::Const(ConstValue::int(1)));
    assert!(answers[0].preserved && answers[0].stated);
    assert_eq!(
        answers[1].value,
        exact_to_lattice(&ExactValue::from_literal("9"))
    );
    assert!(!answers[1].preserved && answers[1].stated);
}

struct PositionedExpressionInputs<'a> {
    inner: &'a dyn AnalysisInputs,
    config: LexerConfig,
    calls: RefCell<Vec<(String, u32, u32)>>,
    prepared: RefCell<Option<ExprNode>>,
}

impl AnalysisInputs for PositionedExpressionInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        self.inner.invocation()
    }
    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        self.inner.operand(id, domain)
    }
    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        self.inner.place(id)
    }
    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        if name == "a{b" && domain == FactDomain::ExactValue {
            FactView::Exact(ExactValue::from_literal("7"), None)
        } else {
            self.inner.variable(name, domain)
        }
    }
    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.inner.prior_store(place, domain)
    }
    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        self.inner.word_structure(id)
    }
    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        self.inner.body(id)
    }
    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        panic!("positioned expression must retain its actual parser extent")
    }
    fn nested_expression_at(
        &self,
        script: &str,
        start: u32,
        end: u32,
        _state: &mut EvaluationState,
    ) -> EvalAnswer {
        self.calls.borrow_mut().push((script.into(), start, end));
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(ExactValue::from_literal("7")),
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence::default(),
        }))
    }
    fn source_lexer_config(&self) -> Option<LexerConfig> {
        Some(self.config)
    }
    fn prepare_expression_source(&self, expression: &ExprNode) {
        *self.prepared.borrow_mut() = Some(expression.clone());
    }
    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.inner.math_function(name)
    }
    fn context(&self) -> &AnalysisContext {
        self.inner.context()
    }
}

#[test]
fn expression_services_forward_full_config_and_inclusive_unicode_child_positions() {
    // naming.expression.original-positioned-analysis-services
    // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
    // Test inputs supply a software result; this proves callback/config transport only.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let inner = tcl_registry::value_transfer::LiteralInputs::new("expr", None, &[], Some(profile));
    let config = LexerConfig {
        expand_syntax: false,
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..LexerConfig::for_profile(Some(profile))
    };
    let inputs = PositionedExpressionInputs {
        inner: &inner,
        config,
        calls: RefCell::new(Vec::new()),
        prepared: RefCell::new(None),
    };
    let identity = ValueIdentity(1);
    let member = ExactValue::from_literal("unused");
    let pinned = tcl_registry::value_transfer::PinnedInputs::new(&inputs, identity, &member);
    assert_eq!(pinned.source_lexer_config(), Some(config));
    for (text, result, expected) in [
        ("[leaf] + 1", "8", Some((0, 5))),
        (r#""é:[leaf]" eq "é:7""#, "1", Some((4, 9))),
        (r#""${a{b}" eq "7""#, "1", None),
    ] {
        inputs.calls.borrow_mut().clear();
        let node = crate::expr_parser::parse_expr_with_syntax_context(
            text,
            &expression_parse_context(&pinned),
        );
        let mut budget = Budget::evaluation();
        let evaluation = ExpressionEvaluation {
            expression: Expression::Parsed(&node),
            policy: FoldPolicy::for_profile(None, Some(profile)),
            nested: NestedPolicy::EffectFreeOnly,
            head: None,
        };
        let answer = evaluation.evaluate(&pinned, &mut budget);
        let EvalAnswer::Evaluated(outcome) = answer else {
            panic!("actual grammar services: {text}: {answer:?}")
        };
        let ExactValueOrUnavailable::Exact(value) = outcome.result else {
            panic!("software result")
        };
        assert_eq!(value.bytes, result.as_bytes());
        assert_eq!(inputs.prepared.borrow().as_ref(), Some(&node));
        let calls = inputs.calls.borrow();
        assert_eq!(
            calls.as_slice(),
            expected.map_or_default(|(start, end)| vec![("leaf".into(), start, end)])
        );
    }
    assert_eq!(
        profile.grammar.braced_var,
        tcl_dialect::BracedVarStyle::Tcl9Nesting
    );
}

/// Inputs whose nested script `boom` raises an error after the writes
/// made so far, and which answer every other request through `inner`.
struct RaisesOnBoom<'a>(&'a dyn AnalysisInputs);

impl AnalysisInputs for RaisesOnBoom<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        self.0.invocation()
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        self.0.operand(id, domain)
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        self.0.place(id)
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        self.0.variable(name, domain)
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.0.prior_store(place, domain)
    }

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        self.0.word_structure(id)
    }

    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        self.0.body(id)
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        if script != "boom" {
            return self.0.nested(script, state);
        }
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Error {
                written: 0,
                message: ExactValueOrUnavailable::Exact(ExactValue::text("mid")),
                error_code: ExactValueOrUnavailable::Exact(ExactValue::text("NONE")),
            },
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::Exact(ExactValue::text("")),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence::default(),
        }))
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.0.math_function(name)
    }

    fn context(&self) -> &AnalysisContext {
        self.0.context()
    }
}

/// An error completion ends the evaluation with that completion and the
/// writes so far: `[incr x] + [boom]` over `x` = 1 leaves the increment
/// in the state and no value, and the error names the writes that ran
/// before it. The lattice publishes nothing for it: the outcome has no
/// exact result.
#[test]
fn an_error_completion_ends_the_evaluation_with_the_writes_so_far() {
    let registry = CommandRegistry::build_default();
    let mutations = crate::command_binding::ModuleCommandMutations::default();
    let driver = LatticeDriver::detached(
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: crate::sccp::FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        FoldPolicy::default(),
    );
    let node = crate::expr_parser::parse_expr_for_profile("[incr x] + [boom] + [incr x]", None);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(&node),
        policy: FoldPolicy::default(),
        nested: NestedPolicy::LocalWrites,
        head: None,
    };
    let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
    let x = ssa.intern_var("x");
    let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1)]);
    let values: HashMap<ValueKey, LatticeValue> =
        HashMap::from([((x, 1), LatticeValue::Const(ConstValue::int(1)))]);
    let inputs = LatticeInputs {
        original: None,
        written_arguments: Vec::new(),
        expression_base: None,
        expression_source: RefCell::new(None),
        nested_source: RefCell::new(None),
        driver: &driver,
        prior_writes: Vec::new(),
        words: Words::Independent,
        view: ResolvedInvocationView {
            canonical_command: "expr",
            subcommand: None,
            form: None,
            layout: InvocationLayout::Source,
            operands: Vec::new(),
            argument_offset: 0,
            arity: None,
        },
        uses: &uses,
        values: &values,
        ssa: &ssa,
        point: None,
        sources: Vec::new(),
    };
    let EvalAnswer::Evaluated(outcome) =
        expression.evaluate(&RaisesOnBoom(&inputs), &mut driver.budget())
    else {
        panic!("the evaluation ends with an outcome");
    };
    assert!(
        matches!(
            &outcome.completion,
            CompletionOutcome::Error { written: 1, message: ExactValueOrUnavailable::Exact(m), .. }
                if m.bytes == b"mid"
        ),
        "{:?}",
        outcome.completion
    );
    assert!(matches!(
        outcome.result,
        ExactValueOrUnavailable::Unavailable(_)
    ));
    let writes: Vec<(&str, &StoreOutcome)> = outcome
        .nested_writes
        .iter()
        .map(|(place, store)| (place.name.as_str(), store))
        .collect();
    assert!(
        matches!(
            writes.as_slice(),
            [("x", StoreOutcome::Write { value, .. })] if value.as_int() == Some(2)
        ),
        "{writes:?}"
    );
}

/// An integer division or remainder by zero is an error completion with
/// the wording every release shares — `divide by zero`, `-errorcode`
/// `ARITH DIVZERO {divide by zero}` — and the writes the expression's own
/// commands made before it are counted. A float operand makes the
/// operation floating point, which does not raise, and an operand that is
/// no integer raises another error the engine does not word.
#[test]
fn an_integer_division_by_zero_is_an_error_completion() {
    let completion_of = |text: &str| {
        let LiftedAnswer::Evaluated(outcomes) = evaluate_over_x(text, NestedPolicy::LocalWrites)
        else {
            panic!("{text} evaluates");
        };
        let [outcome] = outcomes.as_slice() else {
            panic!("one outcome for {text}: {outcomes:?}");
        };
        (outcome.completion.clone(), outcome.nested_writes.len())
    };
    let divide_by_zero = |written: usize| CompletionOutcome::Error {
        written,
        message: ExactValueOrUnavailable::exact_text("divide by zero"),
        error_code: ExactValueOrUnavailable::exact_text("ARITH DIVZERO {divide by zero}"),
    };
    for (text, written) in [
        ("1 / 0", 0),
        ("$x % 0", 0),
        ("0 / 0", 0),
        ("[incr x] / 0", 1),
        ("[incr x] + [incr x] % 0", 2),
    ] {
        let (completion, writes) = completion_of(text);
        assert_eq!(completion, divide_by_zero(written), "{text}");
        assert_eq!(writes, written, "{text}");
    }
    for text in ["1 / 0.0", "1.0 / 0", "$x / 2", "1 % 3"] {
        let (completion, _) = completion_of(text);
        assert_eq!(completion, CompletionOutcome::Normal, "{text}");
    }
    assert!(
        matches!(
            evaluate_over_x("{a} / 0", NestedPolicy::LocalWrites),
            LiftedAnswer::Declined(_)
        ),
        "no integer is divided"
    );
}

#[test]
fn lattice_constants_round_trip_through_exact_values() {
    for c in [
        ConstValue::int(42),
        ConstValue::int(-7),
        ConstValue::float(1.5),
        ConstValue::boolean(true),
        ConstValue::String(" a ".into()),
        ConstValue::String("a\0b".into()),
        ConstValue::String("a\\b".into()),
        ConstValue::String("08".into()),
    ] {
        assert_eq!(exact_to_const(&const_to_exact(&c)), Some(c));
    }
}

#[test]
fn lattice_projection_keeps_noncanonical_numeric_bytes_and_declines_nontext() {
    use tcl_registry::value_transfer::NumericValue;

    let mut value = ExactValue::text("010");
    value.numeric = Some(NumericValue::Int(10));
    assert_eq!(
        exact_to_const(&value),
        Some(ConstValue::String("010".into()))
    );
    value.bytes = vec![0xff];
    assert_eq!(exact_to_lattice(&value), LatticeValue::Overdefined);
}

#[test]
fn lattice_text_joins_equal_contents_without_retaining_native_representation() {
    use tcl_registry::value_transfer::RepresentationEvidence;

    let text = ExactValue::text("key value");
    let mut list = text.clone();
    list.representation = RepresentationEvidence::Constructed(tcl_registry::TclType::List);
    let joined = crate::sccp::join(&exact_to_lattice(&text), &exact_to_lattice(&list));
    let LatticeValue::Const(value) = joined else {
        panic!("equal contents should remain one constant, got {joined:?}");
    };
    assert_eq!(const_to_exact(&value), text);
}

#[test]
fn source_word_reads_keep_literal_colons_and_selected_unicode_grammar() {
    // Original inputs: experiments/name-handling/oracles/whole-variable-reference-shapes-20261014.
    use crate::static_loops::{LoopState, Slot};
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let context = tcl_registry::model::ingress::static_context_for(profile);
        let driver = LatticeDriver::over_registry(
            context.commands(),
            FoldPolicy::for_profile(None, context.commands().profile()),
        );
        let mut state = LoopState::default();
        for (name, value) in [
            ("a", "A"),
            ("a:b", "AB"),
            ("caf", "PREFIX"),
            ("café", "UNICODE"),
            ("", "EMPTY"),
            ("scalar(open", "OPEN"),
            ("a{b", "FIRST"),
            ("a{b}c", "NESTED"),
        ] {
            state.set(
                name.to_owned(),
                Slot::Value(ExactValue::text(value.to_owned())),
            );
        }
        for (source, expected) in [
            ("$a:b", "A:b"),
            ("${}", "EMPTY"),
            ("${scalar(open}", "OPEN"),
            (
                "$café",
                if profile == "jim" {
                    "UNICODE"
                } else {
                    "PREFIXé"
                },
            ),
            (
                "${a{b}c}",
                if matches!(profile, "tcl9.0" | "tcl9.1") {
                    "NESTED"
                } else {
                    "FIRSTc}"
                },
            ),
        ] {
            assert_eq!(
                driver
                    .word_in_state(&state, source, false)
                    .map(|value| value.bytes),
                Ok(expected.as_bytes().to_vec()),
                "{profile}: {source}"
            );
        }
        for source in ["${missing}", "${a(key)}", "$a(key)"] {
            assert!(
                driver.word_in_state(&state, source, false).is_err(),
                "{profile}: {source}"
            );
        }
        assert_eq!(
            driver
                .word_in_state(&state, "$a:b", true)
                .map(|value| value.bytes),
            Ok(b"$a:b".to_vec())
        );
    }
}

/// A whole reference retains its selected release's closing rule. A lowered
/// quoted `expr` word spells `${a} + ${b}`, which starts and ends like one
/// braced reference and is not the variable `a} + ${b`. The `${…}`
/// closer is the release rule's: `${a{b}c}` is one reference
/// under 9.x and `${a{b}` followed by `c}` under 8.x.
#[test]
fn a_simple_reference_is_one_reference_only() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let config = |style| LexerConfig {
        braced_var: style,
        ..LexerConfig::default()
    };
    for style in [Tcl9Nesting, FirstClose] {
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("$a", config(style)),
            Some("a")
        );
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("${a b}", config(style)),
            Some("a b")
        );
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("$::ns::v", config(style)),
            Some("::ns::v")
        );
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("${a} + ${b}", config(style)),
            None
        );
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("$a + $b", config(style)),
            None
        );
        assert_eq!(
            tcl_syntax::naming::whole_variable_reference_literal_name("a", config(style)),
            None
        );
    }
    assert_eq!(
        tcl_syntax::naming::whole_variable_reference_literal_name("${a{b}c}", config(Tcl9Nesting)),
        Some("a{b}c")
    );
    assert_eq!(
        tcl_syntax::naming::whole_variable_reference_literal_name("${a{b}c}", config(FirstClose)),
        None
    );
    assert_eq!(
        tcl_syntax::naming::whole_variable_reference_literal_name("${a\\}", config(FirstClose)),
        Some("a\\")
    );
    assert_eq!(
        tcl_syntax::naming::whole_variable_reference_literal_name("${a\\}", config(Tcl9Nesting)),
        None
    );
}

/// A `Raw` operand reads a variable only when it is one reference whose
/// name the release rules read alike: `${acc}` and `$acc` do, a
/// backslash or brace in the name never does, and neither does any
/// other `Raw` text.
#[test]
fn a_raw_operand_is_a_variable_only_when_every_rule_agrees() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    let config = |style| LexerConfig {
        braced_var: style,
        ..LexerConfig::default()
    };
    for style in [Tcl9Nesting, FirstClose] {
        assert_eq!(whole_variable_operand("${acc}", config(style)), Some("acc"));
        assert_eq!(whole_variable_operand("$acc", config(style)), Some("acc"));
        assert_eq!(whole_variable_operand("${a\\b}", config(style)), None);
        assert_eq!(whole_variable_operand("<switch_jump>", config(style)), None);
        assert_eq!(whole_variable_operand("$a + $b", config(style)), None);
    }
    assert_eq!(
        whole_variable_operand("${a{b}c}", config(Tcl9Nesting)),
        None
    );
}

/// A condition's truth as `if` reads it: a number is true when non-zero,
/// a boolean word is its value, and a beyond-wide integer's canonical
/// spelling is true when non-zero. A digit string with a leading zero is
/// no canonical spelling: under 8.x it reached the condition as text
/// because the grammar read it as an invalid octal, and `if {08}` raises
/// `expected boolean value but got "08"` on tclsh 8.5 and 8.6.
#[test]
fn a_condition_reads_only_canonical_digits_as_a_number() {
    let text = |t: &str| ExactValueOrUnavailable::Exact(ExactValue::text(t));
    assert_eq!(truth_of(&text("yes")), Some(true));
    assert_eq!(truth_of(&text("off")), Some(false));
    assert_eq!(truth_of(&text("123456789012345678901234")), Some(true));
    assert_eq!(truth_of(&text("-123456789012345678901234")), Some(true));
    assert_eq!(truth_of(&text("0")), Some(false));
    for undecided in ["08", "-08", "007", "00", "x"] {
        assert_eq!(truth_of(&text(undecided)), None, "{undecided}");
    }
    assert_eq!(
        truth_of(&ExactValueOrUnavailable::Exact(ExactValue::int(0))),
        Some(false)
    );
}
