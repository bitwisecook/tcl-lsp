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
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

#[test]
fn operand_collection_accepts_fixed_strings_without_donating_numeric_results() {
    let profile = tcl_registry::model::ingress::static_context_for("tcl8.6")
        .commands()
        .profile()
        .expect("actual Tcl 8.6 fixture");
    let tree = crate::parse_expr_for_profile("$x == \"hello\"", Some(profile));
    let mut reads = 0;
    assert!(retained_operand_tree(&tree, |_| {
        reads += 1;
        true
    }));
    assert_eq!(reads, 1);
    assert!(!numeric_tree(&tree, true, |_| true));
    for source in ["$x == \"$y\"", "$x == \"[callback]\"", "$x == \"he\\llo\""] {
        let tree = crate::parse_expr_for_profile(source, Some(profile));
        assert!(!retained_operand_tree(&tree, |_| true), "{source}");
    }
}

fn inspect_read<T>(
    source: &str,
    profile: &str,
    inspect: impl FnOnce(
        &crate::command_binding::SourceVariableAccess,
        &tcl_registry::CommandRegistry,
    ) -> T,
) -> T {
    let context = tcl_registry::model::ingress::static_context_for(profile);
    let registry = context.commands();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
    let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
    assert_eq!(reads.len(), 1, "retained original x read: {profile}");
    inspect(&reads[0], registry)
}

fn read_proof(
    source: &str,
    profile: &str,
) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof> {
    inspect_read(source, profile, source_read_operand)
}

#[test]
fn joined_numeric_producers_retain_only_their_closed_current_category() {
    use tcl_registry::TclType;
    for (left, right, expected) in [
        ("[llength {a b}]", "[expr {$raw<<1}]", Some(TclType::Int)),
        (
            "[expr {sqrt(4)}]",
            "[expr {sqrt(9)}]",
            Some(TclType::Double),
        ),
        (
            "[llength {a b}]",
            "[expr {sqrt(9)}]",
            Some(TclType::Numeric),
        ),
        ("[llength {a b}]", "2", None),
        ("[llength {a b}]", "[opaque]", None),
    ] {
        let source = format!(
            "proc f {{condition raw}} {{if {{$condition}} {{set x {left}}} else {{set x {right}}}; set view $x}}"
        );
        inspect_read(&source, "tcl8.6", |read, registry| {
            assert_ne!(read.context_alternatives(), []);
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert_eq!(
                    context.contents_native_numeric_category_at(&place, registry),
                    expected,
                    "{source}: {:?}",
                    context.value_representations
                );
                assert!(
                    context
                        .contents_native_numeric_at(&place, registry)
                        .is_none()
                );
                assert_eq!(
                    context.contents_native_string_access_closed_at(&place, registry),
                    right != "[opaque]" && right != "2",
                    "{source}"
                );
            }
            assert!(source_read_operand(read, registry).is_none());
        });
    }
}

#[test]
fn executed_arithmetic_result_shape_is_independent_of_operand_erasure() {
    for profile in ["tcl8.6", "tcl9.1"] {
        for (expression, expected) in [
            ("$raw+0", true),
            ("$raw<<1", true),
            ("~$raw", true),
            ("$raw==7", false),
            ("$raw||1", false),
            ("!$raw", false),
            ("2+2", false),
            ("$raw", false),
        ] {
            let source = format!("proc f {{raw}} {{set x [expr {{{expression}}}]; expr {{$x+1}}}}");
            inspect_read(&source, profile, |read, registry| {
                for context in read.context_alternatives() {
                    let place = crate::var_resolve::resolve_substitution_access(
                        &read.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    assert_eq!(
                        context.contents_already_native_numeric_at(&place, registry),
                        expected,
                        "{profile}: {expression}: {:?}",
                        context.value_representations
                    );
                    assert!(
                        context
                            .contents_native_numeric_at(&place, registry)
                            .is_none()
                    );
                }
                assert!(source_read_operand(read, registry).is_none());
            });
        }
    }
}

#[test]
fn a_coercing_write_callback_retires_an_arithmetic_result_shape() {
    let source = "proc observe args {upvar 1 x slot; llength $slot}; proc f {raw} {trace add variable x write observe; set x [expr {$raw+0}]; expr {$x+1}}";
    inspect_read(source, "tcl8.6", |read, registry| {
        for context in read.context_alternatives() {
            let place = crate::var_resolve::resolve_substitution_access(
                &read.original_spelling,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert!(!context.contents_already_native_numeric_at(&place, registry));
        }
    });
}

#[test]
fn pooled_native_result_contents_do_not_mint_a_numeric_representation() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (expression, expected) in [("2+2", "4"), ("false", "false")] {
            let source = format!("set x [expr {{{expression}}}]; expr {{$x+1}}");
            inspect_read(&source, profile, |read, registry| {
                assert!(source_read_operand(read, registry).is_none());
                let [context] = read.context_alternatives() else {
                    panic!("one retained physical read");
                };
                let place = crate::var_resolve::resolve_substitution_access(
                    &read.original_spelling,
                    context,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                assert_eq!(
                    context.literal_contents_at(&place, registry),
                    Some(expected),
                    "{profile}: native normal result bytes remain independent of pool representation"
                );
            });
        }
    }
}

#[test]
fn source_numeric_result_retains_native_representation_at_actual_read() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let proof = read_proof("set x [expr {4}]; expr {$x + 1}", profile).expect(profile);
        assert_eq!(proof.value, TclValue::Int(4));
        assert!(matches!(
            proof.identity,
            crate::tcl_expr_eval::NativeOperandObjectIdentity::Source { .. }
        ));
    }
}

#[test]
fn normalised_result_strings_prove_only_literal_pool_disjointness() {
    for profile in ["tcl8.6", "tcl9.0", "jim"] {
        for (expression, expected) in [("$raw+0", true), ("+$raw", false), ("$raw**1", false)] {
            let source = format!(
                "proc f {{raw}} {{set x [expr {{{expression}}}]; expr {{$x == \"hello\"}}}}"
            );
            inspect_read(&source, profile, |read, registry| {
                for context in read.context_alternatives() {
                    let place = read.place_in_context(context, registry);
                    assert_eq!(
                        context.contents_already_native_numeric_at(&place, registry),
                        expected,
                        "{profile}: {expression}: {:?}",
                        context.value_representations
                    );
                    assert!(context.literal_contents_at(&place, registry).is_none());
                    assert!(source_read_operand(read, registry).is_none());
                    if expected {
                        let mut overlapping = context.as_ref().clone();
                        overlapping.invalidate_representations_for_values(Some(&["7".to_owned()]));
                        assert!(!overlapping.contents_already_native_numeric_at(&place, registry));
                        let mut unknown = context.as_ref().clone();
                        unknown.invalidate_shared_representations();
                        assert!(!unknown.contents_already_native_numeric_at(&place, registry));
                    }
                }
            });
        }
    }
}

#[test]
fn integer_contents_join_closes_conversion_without_an_intrep_or_value() {
    let source = "proc f {flag x} {if {$flag} {set x 0; llength $x} else {incr x}; puts $x}";
    inspect_read(source, "tcl8.6", |read, registry| {
        assert_ne!(read.context_alternatives(), []);
        for context in read.context_alternatives() {
            let place = read.place_in_context(context, registry);
            assert!(
                context
                    .contents_integer_increment_conversion_at(&place, registry)
                    .is_some()
            );
            assert!(!context.contents_already_native_numeric_at(&place, registry));
            assert!(context.literal_contents_at(&place, registry).is_none());
            assert!(source_read_operand(read, registry).is_none());
            let mut unknown = context.as_ref().clone();
            unknown.invalidate_shared_representations();
            assert!(
                unknown
                    .contents_integer_increment_conversion_at(&place, registry)
                    .is_none()
            );
        }
    });
    for source in [
        "proc f {flag x} {if {$flag} {set x 0} else {incr x}; puts $x}",
        "proc f {flag x} {if {$flag} {set x 0.5} else {incr x}; puts $x}",
        "proc f {flag x} {if {$flag} {set x hello} else {incr x}; puts $x}",
        "proc f {flag} {if {$flag} {set x 0}; puts $x}",
    ] {
        inspect_read(source, "tcl8.6", |read, registry| {
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert!(
                    context
                        .contents_integer_increment_conversion_at(&place, registry)
                        .is_none(),
                    "{source}"
                );
            }
        });
    }
}

#[test]
fn stock_length_cache_requires_the_selected_original_class() {
    // Same-object probes include Tcl 9 numeric Length preservation:
    // tcl-syntax/tests/data/native_list_methods/stock_length/README.md.
    for (profile, expected) in [
        ("tcl8.4", tcl_syntax::value::ValueRepresentation::List),
        ("tcl8.5", tcl_syntax::value::ValueRepresentation::List),
        ("tcl8.6", tcl_syntax::value::ValueRepresentation::List),
        ("tcl9.0", tcl_syntax::value::ValueRepresentation::Unknown),
        ("tcl9.1", tcl_syntax::value::ValueRepresentation::Unknown),
    ] {
        inspect_read("set x 0; llength $x; puts $x", profile, |read, registry| {
            assert_ne!(read.context_alternatives(), []);
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert_eq!(
                    context.contents_representation_at(&place),
                    expected,
                    "{profile}: {:?}",
                    context.value_representations
                );
                assert!(
                    context
                        .contents_stock_literal_object_at(&place, registry)
                        .is_some(),
                    "{profile}"
                );
                assert!(!context.contents_already_native_numeric_at(&place, registry));
                assert!(source_read_operand(read, registry).is_none());
            }
        });
    }
}

#[test]
fn normal_length_preserves_only_the_audited_numeric_cache() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        inspect_read(
            "proc f {x} {set ordinary [list $x]; set x [llength $ordinary]; incr x; llength $x; puts $x}",
            profile,
            |read, registry| {
                assert_ne!(read.context_alternatives(), []);
                let preserves = matches!(profile, "tcl9.0" | "tcl9.1");
                for context in read.context_alternatives() {
                    let place = read.place_in_context(context, registry);
                    assert_eq!(
                        context.contents_already_native_numeric_at(&place, registry),
                        preserves,
                        "{profile}: {:?}",
                        context.value_representations
                    );
                    assert_eq!(
                        context.contents_representation_at(&place),
                        if preserves {
                            tcl_syntax::value::ValueRepresentation::Unknown
                        } else {
                            tcl_syntax::value::ValueRepresentation::List
                        },
                        "{profile}"
                    );
                    // Cache preservation retires concrete operand/epoch receipts.
                    assert!(source_read_operand(read, registry).is_none());
                }
            },
        );
    }
}

#[test]
fn empty_root_join_closes_methods_without_a_container_cache() {
    use tcl_registry::native_result::NativeListMethodProvider::EmptyListRoot;
    let empty = StoredNativeRepresentation::ReadOnlyListMethodProvider(EmptyListRoot);
    let list = StoredNativeRepresentation::Known(tcl_syntax::value::ValueRepresentation::List);
    for joined in [empty.joined(&list), list.joined(&empty)] {
        let joined = joined.unwrap();
        assert_eq!(joined, empty);
        assert_eq!(
            joined.representation(),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
        assert!(joined.container_alternatives().is_none());
        assert!(!EmptyListRoot.world_is_closed_for(
            tcl_registry::list_object_methods::NativeListMethod::StringAccess
        ));
        assert!(!EmptyListRoot.length_is_read_only());
    }
    assert!(
        empty
            .joined(&StoredNativeRepresentation::Known(
                tcl_syntax::value::ValueRepresentation::Dict
            ))
            .is_none()
    );
}

#[test]
fn accepted_integer_contents_are_not_current_numeric_object_proofs() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        inspect_read("set x 005; puts $x", profile, |read, registry| {
            let receipt = source_read_integer_contents(read, registry).unwrap();
            assert_eq!(receipt.contents(), "005");
            assert_eq!(receipt.number(), &TclValue::Int(5));
            assert!(source_read_operand(read, registry).is_none());
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert!(!context.contents_already_native_numeric_at(&place, registry));
            }

            let policy = crate::tcl_expr_eval::FoldPolicy::from_registry(registry);
            for (expression, closed) in [("$x * 2", true), ("!$x", false), ("$x", false)] {
                let tree = crate::parse_expr_for_profile(expression, registry.profile());
                let (env, operands, contents) = expression_operands_with_integer_contents(
                    &tree,
                    |_| None,
                    |_| Some(receipt.clone()),
                    |_| false,
                )
                .unwrap();
                assert!(operands.is_empty());
                let result = crate::tcl_expr_eval::analyse_tcl_expr_with_integer_contents(
                    &tree,
                    &env,
                    policy,
                    &|_, _| None,
                    &operands,
                    &contents,
                )
                .unwrap();
                assert_eq!(
                    result.native_value_effects_are_proved(),
                    closed,
                    "{profile}: {expression}"
                );
                if closed {
                    assert_eq!(result.value, TclValue::Int(10));
                    assert!(result.result_dependency.is_none());
                }
                let wrong_env = crate::tcl_expr_eval::Env::from([(
                    "x".into(),
                    crate::tcl_expr_eval::EnvValue::text("6"),
                )]);
                assert!(
                    crate::tcl_expr_eval::analyse_tcl_expr_with_integer_contents(
                        &tree,
                        &wrong_env,
                        policy,
                        &|_, _| None,
                        &operands,
                        &contents,
                    )
                    .is_none()
                );
            }
        });
    }
}

#[test]
fn accepted_integer_contents_require_original_source_value_and_selected_recipe() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    for source in [
        "set x 1.0; puts $x",
        "set x [expr {double(1)}]; puts $x",
        "proc f {x} {puts $x}",
        "set x 21; set ::auto_index(unknown_host) {source lib.tcl}; unknown_host; puts $x",
    ] {
        inspect_read(source, "tcl8.6", |read, registry| {
            assert!(
                source_read_integer_contents(read, registry).is_none(),
                "{source}"
            );
        });
    }
    for profile in ["tcl8.4", "f5-irules"] {
        inspect_read("set x 21; puts $x", profile, |read, registry| {
            assert!(
                source_read_integer_contents(read, registry).is_none(),
                "{profile}"
            );
        });
    }
    inspect_read("set x 21; puts $x", "tcl8.6", |read, registry| {
        let receipt = source_read_integer_contents(read, registry).unwrap();
        assert!(!receipt.accepts_number(None, "21"));
        assert!(!receipt.accepts_number(
            Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_0
            )),
            "21"
        ));
        let mut foreign = receipt.clone();
        foreign.cells.clear();
        assert!(!receipt.agrees_with(&foreign));
    });
}

#[test]
fn fresh_stock_integer_contents_survive_without_current_integer_cache() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        inspect_read(
            "set x 0; expr {$x < 3}; puts $x",
            profile,
            |read, registry| {
                assert_ne!(read.context_alternatives(), []);
                for context in read.context_alternatives() {
                    let place = read.place_in_context(context, registry);
                    assert!(
                        context
                            .contents_integer_increment_conversion_at(&place, registry)
                            .is_some()
                    );
                    assert_ne!(
                        context.contents_native_numeric_category_at(&place, registry),
                        Some(tcl_registry::TclType::Int)
                    );
                    assert!(source_read_operand(read, registry).is_none());
                }
            },
        );
    }
    inspect_read(
        "proc f {x} {expr {$x < 3}; puts $x}",
        "tcl8.6",
        |read, registry| {
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert!(
                    context
                        .contents_integer_increment_conversion_at(&place, registry)
                        .is_none()
                );
            }
        },
    );
}

#[test]
fn numeric_conversion_retains_only_disjoint_stock_cache_history() {
    inspect_read(
        "set x 0; llength $x; puts $x",
        "tcl8.6",
        |read, registry| {
            for context in read.context_alternatives() {
                let place = read.place_in_context(context, registry);
                assert_eq!(
                    context
                        .contents_stock_literal_object_at(&place, registry)
                        .expect("actual original literal class")
                        .representation(),
                    tcl_syntax::value::ValueRepresentation::List
                );
                for (values, expected) in [
                    (
                        Some(vec!["1".to_owned()]),
                        tcl_syntax::value::ValueRepresentation::List,
                    ),
                    (
                        Some(vec!["0".to_owned()]),
                        tcl_syntax::value::ValueRepresentation::Unknown,
                    ),
                    (None, tcl_syntax::value::ValueRepresentation::Unknown),
                ] {
                    let mut converted = context.as_ref().clone();
                    converted.invalidate_numeric_conversion_representations(values.as_deref());
                    assert_eq!(
                        converted
                            .contents_stock_literal_object_at(&place, registry)
                            .expect("ordinary class only")
                            .representation(),
                        expected
                    );
                    // The literal `0` converts as an integer whatever its
                    // cached representation: tclsh 8.4.20 to 9.1.0 print `1`
                    // for `set x 0; llength $x; puts [incr x]`.
                    assert!(
                        converted
                            .contents_integer_increment_conversion_at(&place, registry)
                            .is_some()
                    );
                    assert!(
                        converted
                            .contents_native_numeric_at(&place, registry)
                            .is_none()
                    );
                }
            }
        },
    );
}

#[test]
fn pooled_literal_strings_do_not_acquire_numeric_object_proof() {
    assert!(read_proof("set x 4; expr {$x + 1}", "tcl9.0").is_none());
}

#[test]
fn unknown_increment_contents_prove_only_current_numeric_shape() {
    inspect_read(
        "proc f {x} {set ordinary [list $x]; set x [llength $ordinary]; incr x; expr {$x+1}}",
        "tcl8.6",
        |read, registry| {
            let [context] = read.context_alternatives() else {
                panic!("one retained physical formal read");
            };
            let place = crate::var_resolve::resolve_substitution_access(
                &read.original_spelling,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert!(
                context.contents_already_native_numeric_at(&place, registry),
                "place={place:?} success={} presence={:?} origin={:?} kind={:?} dialect={:?} repr={:?}",
                context.read_produces_value(&place, registry),
                context.contents_presence(&place),
                context.contents_origin(&place),
                context.root_contents_kind(&place),
                context.invocation_dialect,
                context.value_representations
            );
            assert!(
                context
                    .contents_native_numeric_at(&place, registry)
                    .is_none()
            );
            assert!(source_read_operand(read, registry).is_none());
            assert_eq!(
                context.contents_representation_at(&place),
                tcl_syntax::value::ValueRepresentation::Unknown
            );
            let mut coerced = context.as_ref().clone();
            coerced.invalidate_shared_representations();
            assert!(!coerced.contents_already_native_numeric_at(&place, registry));
        },
    );
}

#[test]
fn unknown_list_length_result_proves_shape_without_numeric_contents() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        inspect_read(
            "proc f {items} {set ordinary [dict create k $items]; set count [llength $ordinary]; set x $count; expr {$x+1}}",
            profile,
            |read, registry| {
                for context in read.context_alternatives() {
                    let place = crate::var_resolve::resolve_substitution_access(
                        &read.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    assert!(
                        context.contents_already_native_numeric_at(&place, registry),
                        "{profile}: {place:?} {:?}",
                        context.value_representations
                    );
                    assert!(context.literal_contents_at(&place, registry).is_none());
                    assert!(
                        context
                            .contents_native_numeric_at(&place, registry)
                            .is_none()
                    );
                    assert_eq!(
                        context.contents_representation_at(&place),
                        tcl_syntax::value::ValueRepresentation::Unknown
                    );
                }
                assert!(source_read_operand(read, registry).is_none());
            },
        );
    }
}

#[test]
fn native_count_shape_follows_only_actual_value_formals() {
    for (actual, expected) in [("[llength {a b}]", true), ("2", false)] {
        for source in [
            format!("proc p {{x}} {{expr {{$x+1}}}}; p {actual}"),
            format!("apply {{{{x}} {{expr {{$x+1}}}}}} {actual}"),
        ] {
            inspect_read(&source, "tcl8.6", |read, registry| {
                for context in read.context_alternatives() {
                    let place = crate::var_resolve::resolve_substitution_access(
                        &read.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    assert_eq!(
                        context.contents_already_native_numeric_at(&place, registry),
                        expected,
                        "{actual}: {:?}",
                        context.value_representations
                    );
                }
                assert!(source_read_operand(read, registry).is_none());
            });
        }
    }
}

#[test]
fn a_count_result_shape_retires_after_a_coercing_write_callback() {
    let source = "proc observe args {upvar 1 x slot; llength $slot}; proc f {items} {trace add variable x write observe; set x [llength $items]; expr {$x+1}}";
    inspect_read(source, "tcl8.6", |read, registry| {
        for context in read.context_alternatives() {
            let place = crate::var_resolve::resolve_substitution_access(
                &read.original_spelling,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert!(!context.contents_already_native_numeric_at(&place, registry));
        }
    });
}

#[test]
fn an_expression_script_cannot_preserve_an_increment_shape() {
    inspect_read(
        "proc f {x} {incr x; expr {[llength $x]+1}; expr {$x+1}}",
        "tcl8.6",
        |read, registry| {
            for context in read.context_alternatives() {
                let place = crate::var_resolve::resolve_substitution_access(
                    &read.original_spelling,
                    context,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                assert!(!context.contents_already_native_numeric_at(&place, registry));
            }
        },
    );
}

#[test]
fn a_write_callback_can_coerce_an_increment_shape_without_changing_bytes() {
    let source = "proc observe args {upvar 1 x slot; llength $slot}; proc f {x} {trace add variable x write observe; incr x; expr {$x+1}}";
    inspect_read(source, "tcl8.6", |read, registry| {
        for context in read.context_alternatives() {
            let place = crate::var_resolve::resolve_substitution_access(
                &read.original_spelling,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert!(!context.contents_already_native_numeric_at(&place, registry));
        }
        assert!(source_read_operand(read, registry).is_none());
    });
}

#[test]
fn pooled_constant_operator_results_do_not_acquire_numeric_representation() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(read_proof("set x [expr {2 + 2}]; expr {$x + 1}", profile).is_none());
        assert!(
            read_proof(
                "proc f {} {expr {2+2}}; set held [f]; llength $held; set x [f]; expr {$x+1}",
                profile,
            )
            .is_none()
        );
        assert!(read_proof("set x [expr {false}]; expr {$x+1}", profile).is_none());
    }
}

#[test]
fn numeric_result_normalisation_reestablishes_a_shimmered_literal_result() {
    let proof = read_proof(
        "proc f {} {expr {4}}; set held [f]; llength $held; set x [f]; expr {$x+1}",
        "tcl8.6",
    )
    .expect("the original native numeric conversion still executes");
    assert_eq!(proof.value, TclValue::Int(4));
}

#[test]
fn numeric_producer_chains_require_actual_retained_operands() {
    let proof = read_proof(
        "set y [expr {4}]; set x [expr {$y + 1}]; expr {$x + 1}",
        "tcl9.0",
    )
    .expect("native numeric read and native numeric result");
    assert_eq!(proof.value, TclValue::Int(5));
    assert!(read_proof("set y 4; set x [expr {$y + 1}]; expr {$x + 1}", "tcl9.0",).is_none());
}

#[test]
fn shared_alias_coercion_withdraws_numeric_object_proof() {
    assert!(
        read_proof(
            "set x [expr {4}]; set alias $x; llength $alias; expr {$x + 1}",
            "tcl9.0"
        )
        .is_none()
    );
}

#[test]
fn expression_list_coercion_withdraws_shared_numeric_object_proof() {
    assert!(
        read_proof(
            "set x [expr {4}]; set alias $x; expr {$alias in $alias}; expr {$x + 1}",
            "tcl9.0"
        )
        .is_none()
    );
}

#[test]
fn ordinary_formal_retains_actual_native_numeric_input() {
    let proof = read_proof("proc p {x} {expr {$x + 1}}; p [expr {4}]", "tcl9.0")
        .expect("ordinary Value binding copies the actual numeric object");
    assert_eq!(proof.value, TclValue::Int(4));
    assert!(read_proof("proc p {x} {expr {$x + 1}}; p 4", "tcl9.0").is_none());
}
