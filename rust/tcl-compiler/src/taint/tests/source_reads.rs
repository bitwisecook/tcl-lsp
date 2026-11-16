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

//! Original operand reads and argument-name inventories under the selected grammar.

use super::*;

#[test]
fn original_operand_reference_names_keep_data_and_current_grammar_without_ssa_grants() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let source = TaintSourceContext::standalone(registry, registry.profile());
        let cfg = crate::cfg::Function::new("model", "entry");
        let ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let uses = HashMap::new();
        let taints = HashMap::<ValueKey, TaintLattice>::new();
        let env = TaintScan {
            uses: &uses,
            taints: &taints,
            ssa: &ssa,
            quoted_uses: None,
            source_read: None,
            source,
        };
        for (reference, name) in [
            ("${$literal}", "$literal"),
            ("${}", ""),
            ("${arr($literal)}", "arr"),
        ] {
            let word =
                crate::value_shapes::value_word_with_config(reference, source.config).unwrap();
            assert!(
                operand_read_taint(&env, name, Some(&word)).is_some_and(TaintLattice::is_tainted)
            );
            assert!(operand_read_taint(&env, "donated", Some(&word)).is_none());
            assert!(ssa.var_symbol(name).is_none());
        }
        assert_eq!(
            reference_root_with_config("${a{b}c}", source.config),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
        for unavailable in [
            "prefix${literal}",
            r"\$literal",
            "${literal",
            "${literal\0tail}",
        ] {
            assert!(reference_root_with_config(unavailable, source.config).is_none());
        }
    }
}

#[test]
fn original_sink_read_keeps_missing_ssa_contents_unknown() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("regexp $missing test", true),
        ("regexp {$missing} test", false),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let tokens = unit
            .top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(Statement::tokens)
            .unwrap();
        let uses = HashMap::new();
        let taints = HashMap::<ValueKey, TaintLattice>::new();
        let env = TaintScan {
            uses: &uses,
            taints: &taints,
            ssa: &unit.top_level.ssa,
            quoted_uses: None,
            source_read: None,
            source: TaintSourceContext::for_function(registry, &unit.top_level),
        };
        assert_eq!(
            operand_read_taint(&env, "missing", tokens.words().get(1))
                .is_some_and(TaintLattice::is_tainted),
            expected,
            "{source}"
        );
        assert!(uses.is_empty());
    }
}

#[test]
fn raw_coercion_references_use_current_grammar_and_keep_empty_name_data() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let source = TaintSourceContext::standalone(registry, registry.profile());
        let grammar = source
            .config
            .grammar_over(registry.profile().unwrap().grammar);
        for (text, name) in [
            ("${$literal}", "$literal"),
            ("${}", ""),
            ("$arr(key)", "arr"),
        ] {
            let names = HashSet::from([name.to_owned()]);
            assert!(holds_one_whole_var_ref(text, &names, source.config));
            assert!(!holds_one_whole_var_ref(
                text,
                &HashSet::from(["donated".to_owned()]),
                source.config
            ));
            let mut hits = Vec::new();
            collect_coercion_operands(
                &ExprNode::Raw {
                    text: text.to_owned(),
                },
                false,
                &mut hits,
                0,
                (source.config, grammar),
            );
            assert!(hits.is_empty(), "{label}: {text}");
        }
        let names = HashSet::from(["a{b}c".to_owned()]);
        assert_eq!(
            holds_one_whole_var_ref("${a{b}c}", &names, source.config),
            label == "tcl9.1"
        );
        for text in ["prefix$x", "$x + 1", "${x", r"\$x", "${x\0tail}"] {
            assert!(!holds_one_whole_var_ref(
                text,
                &HashSet::from(["x".to_owned()]),
                source.config
            ));
        }
        let mut hits = Vec::new();
        collect_coercion_operands(
            &ExprNode::Var {
                text: "${}".into(),
                name: String::new(),
                start: 0,
                end: 2,
            },
            true,
            &mut hits,
            0,
            (source.config, grammar),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "");
        assert!(hits[0].variable.is_some());
        assert_eq!(hits[0].rel_span, Some((0, 3)));
    }
}

#[test]
fn original_scalar_reference_consumers_keep_literal_words_and_current_closers() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (spelling, expected) in [
            ("${$literal}", Some("$literal")),
            ("${}", Some("")),
            ("{$literal}", None),
            ("{${literal}}", None),
            ("$arr(key)", None),
            ("${arr(key)}", None),
            ("prefix$x", None),
            (r"\$x", None),
        ] {
            let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
            assert_eq!(
                original_scalar_reference_name(&word, config),
                expected,
                "{label}: {spelling}"
            );
        }
        let cfg = crate::cfg::Function::new("model", "entry");
        let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let symbol = ssa.intern_var("$literal");
        let statement = ssa_stmt(
            Statement::Incr {
                span: Span::new(0, 0),
                name: "$literal".into(),
                name_braced: true,
                amount: None,
                amount_braced: false,
                safe_on_uninit: false,
            },
            HashMap::from([(symbol, 1)]),
            HashMap::new(),
        );
        let taints = HashMap::from([((symbol, 1), TaintLattice::tainted())]);
        for (spelling, expected) in [("${$literal}", 1), ("{$literal}", 0)] {
            let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
            let mut warnings = Vec::new();
            emit_dynamic_command_head_warning(
                Some((&word, config)),
                &statement,
                &taints,
                &mut warnings,
                &ssa,
            );
            assert_eq!(warnings.len(), expected, "{label}: {spelling}");
            if let Some(warning) = warnings.first() {
                assert_eq!(warning.variable, "$literal");
            }
        }
        let mut warnings = Vec::new();
        emit_dynamic_command_head_warning(None, &statement, &taints, &mut warnings, &ssa);
        assert_eq!(warnings, [] as [crate::taint::TaintWarning; 0]);
        let word = crate::value_shapes::value_word_with_config("${a{b}c}", config).unwrap();
        assert_eq!(
            original_scalar_reference_name(&word, config),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
    }
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let config = TaintSourceContext::standalone(registry, registry.profile()).config;
    let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
    for (spelling, name, expected) in [
        ("${$literal}", "$literal", true),
        ("${}", "", true),
        ("{$literal}", "literal", false),
    ] {
        let args = vec![
            "insert".to_owned(),
            spelling.to_owned(),
            "$value".to_owned(),
        ];
        let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
        assert_eq!(
            irule3002_name_position_safe(
                registry,
                "HTTP::header",
                &args,
                name,
                lat,
                Some(&word),
                config
            ),
            expected
        );
        assert!(!irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &args,
            name,
            lat,
            None,
            config
        ));
    }
}

/// The `${…}` closer comes from the shared owner, so the name taint is
/// keyed on is the one the lexer spanned.
///
/// Taint is name-keyed: a scan that recovers `a{b` where the document's
/// lexer spanned `a{b}c` looks up a cell nothing ever writes, so the
/// tainted value's colour is lost and the sink diagnostics that depend on
/// it (T1xx, W313) go quiet on a flow they should flag:
/// `set {a{b}c} 7; subst {${a{b}c}}` is `7` on tclsh 9.0.4 and
/// `can't read "a{b"` on 8.6.16.
#[test]
fn original_argument_candidates_use_full_source_config_and_nested_index_inventory() {
    let context = tcl_registry::model::ingress::static_context_for("tcl9.1");
    let registry = context.commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "puts [list exec $arr($index) ${a{b}c} $tail $tail]",
        registry,
        false,
    );
    let tokens = unit.ir_module.top_level.statements[0].tokens().unwrap();
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert_eq!(source.config, unit.ir_module.lexer_config);
    let names = arg_var_names_with_config(&tokens.argv_texts[1], source.config);
    assert!(!names.has_unknown_residual());
    assert_eq!(names.names(), ["arr", "index", "a{b}c", "tail", "tail"]);
}

#[test]
fn unavailable_argument_inventory_keeps_known_reads_and_refuses_mitigations() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "set value SAFE; puts {[missing $value}",
        registry,
        false,
    );
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let tokens = unit
        .ir_module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let args = tokens.argv_texts[1..].to_vec();
    let call = SinkCall {
        source,
        command: "puts",
        args: &args,
        registry,
        tokens: Some(tokens),
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(arg_var_names_with_config(&args[0], source.config).has_unknown_residual());
    assert!(
        all_argument_candidates(&args[0], source.config, &unit.top_level.ssa)
            .iter()
            .any(|name| name == "value")
    );
    assert!(!var_consumed_by_sanitiser(&call, "value"));
    assert!(!list_wrapped_arg_command_is_literal(&call, "value"));
    assert!(!sink_var_position_safe(&call, DiagCode::T100, "value"));
    assert!(var_wrapper_colours(registry, source, &args, "value", Some(tokens), 0).is_none());
}

#[test]
fn arg_var_names_follows_the_release_close_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    assert_eq!(
        arg_var_names("/tmp/${a{b}c}", Tcl9Nesting),
        HashSet::from(["a{b}c".to_owned()])
    );
    assert_eq!(
        arg_var_names("/tmp/${a{b}c}", FirstClose),
        HashSet::from(["a{b".to_owned()])
    );
    // The scan resumes past the reference under either rule, so a second
    // ordinary reference in the same word is still collected.
    assert_eq!(
        arg_var_names("${a{b}c}/$tail", Tcl9Nesting),
        HashSet::from(["a{b}c".to_owned(), "tail".to_owned()])
    );
}
