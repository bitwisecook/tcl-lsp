// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::{
    CommandRegistry, InvocationDialect, InvocationWord, InvocationWords,
    native_compilation::NativeCompilationGuard,
    runtime_expr_validation::{ExpressionPreparationProof, prepare_expression_witness},
};
use tcl_syntax::expr::parser::ExprParseContext;

const PROFILES: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];
const VERSIONS: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];
const FIXTURES: [&str; 5] = [
    include_str!("../../../tcl-syntax/tests/data/native_numeric_operand_conversions/8.4.20.txt"),
    include_str!("../../../tcl-syntax/tests/data/native_numeric_operand_conversions/8.5.19.txt"),
    include_str!("../../../tcl-syntax/tests/data/native_numeric_operand_conversions/8.6.18.txt"),
    include_str!("../../../tcl-syntax/tests/data/native_numeric_operand_conversions/9.0.4.txt"),
    include_str!("../../../tcl-syntax/tests/data/native_numeric_operand_conversions/9.1.0.txt"),
];

fn witness(profile: &str, source: &str) -> PreparedExpressionWitness {
    let context = crate::model::ingress::static_context_for(profile);
    let syntax = ExprParseContext::for_profile(context.commands().profile().unwrap());
    let ExpressionPreparationProof::Prepared(witness) =
        prepare_expression_witness(source, &syntax, None)
    else {
        panic!("prepared {profile}: {source}");
    };
    *witness
}
fn field<'a>(line: &'a str, name: &str) -> &'a str {
    line.split_ascii_whitespace()
        .find_map(|part| {
            let (key, value) = part.split_once('=')?;
            (key == name).then_some(value)
        })
        .unwrap()
}

#[test]
fn expression_conversion_requires_the_actual_integer_numeric_branch() {
    for profile in PROFILES {
        let prepared = witness(profile, "$i < 3");
        let ExprNode::Binary { left, .. } = prepared.tree() else {
            panic!("comparison");
        };
        let production = prepared.integer_relational_operand_conversion(left);
        assert_eq!(production.is_some(), profile != "tcl8.4", "{profile}");
        if let Some(production) = production {
            assert_eq!(production.category(), TclType::Numeric);
            assert!(!production.normalises_original_string());
            for (class, category) in [
                (NativeNumericOperandClass::String, TclType::Int),
                (NativeNumericOperandClass::List, TclType::Int),
                (NativeNumericOperandClass::ByteArray, TclType::Int),
                (NativeNumericOperandClass::Integer, TclType::Int),
                (NativeNumericOperandClass::Double, TclType::Double),
                (NativeNumericOperandClass::Dict, TclType::Numeric),
                (NativeNumericOperandClass::Numeric, TclType::Numeric),
                (NativeNumericOperandClass::Unknown, TclType::Numeric),
            ] {
                let refined = production.with_original_cache_class(class);
                assert_eq!(refined.category(), category, "{profile}: {class:?}");
                assert!(!refined.normalises_original_string());
            }
        }
        assert!(
            prepared
                .integer_relational_operand_conversion(prepared.tree())
                .is_none()
        );
        for source in ["$i eq 3", "$i == 3", "$i && 3"] {
            let prepared = witness(profile, source);
            let ExprNode::Binary { left, .. } = prepared.tree() else {
                continue;
            };
            assert!(
                prepared
                    .integer_relational_operand_conversion(left)
                    .is_none()
            );
        }
    }
}

#[test]
fn index_conversion_keeps_grouped_objects_and_immediate_indices_separate() {
    let registry = CommandRegistry::build_default();
    for version in VERSIONS {
        let dialect = InvocationDialect::for_version(version);
        for (head, values, index) in [
            (
                "lindex",
                vec![InvocationWord::Literal("a b"), InvocationWord::Dynamic],
                1,
            ),
            (
                "lrange",
                vec![
                    InvocationWord::Literal("a b"),
                    InvocationWord::Dynamic,
                    InvocationWord::Literal("1"),
                ],
                1,
            ),
        ] {
            let words = InvocationWords::structured(InvocationWord::Literal(head), &values)
                .with_dialect(dialect);
            let facts = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .unwrap()
                .facts();
            let production = facts
                .integer_index_operand_conversion(
                    words.arguments(),
                    NativeCompilationSelection::Generic,
                    index,
                    NativeCompilationWordShape::Substituted,
                )
                .unwrap();
            assert_eq!(production.requires_non_list_input(), head == "lindex");
            assert_eq!(production.tcl_version(), version);
            assert_eq!(
                production
                    .with_original_cache_class(NativeNumericOperandClass::Double)
                    .category(),
                TclType::Int,
                "actual normal integer-index continuation is a separate stage"
            );
            let inline = NativeCompilationSelection::Inline {
                operation: facts.operation,
                guard: if version == TclVersion::V8_4 {
                    NativeCompilationGuard::ChunkEntry
                } else {
                    NativeCompilationGuard::BeforeArguments
                },
            };
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::BracedLiteral,
                NativeCompilationWordShape::QuotedLiteral,
                NativeCompilationWordShape::Opaque,
            ] {
                assert!(
                    facts
                        .integer_index_operand_conversion(words.arguments(), inline, index, shape)
                        .is_none()
                );
            }
            assert!(
                facts
                    .integer_index_operand_conversion(
                        words.arguments(),
                        NativeCompilationSelection::Unknown,
                        index,
                        NativeCompilationWordShape::Substituted
                    )
                    .is_none()
            );
            assert!(
                facts
                    .integer_index_operand_conversion(
                        words.arguments(),
                        NativeCompilationSelection::Generic,
                        0,
                        NativeCompilationWordShape::Substituted
                    )
                    .is_none()
            );
        }
    }
}

#[test]
fn jim_index_conversion_declines_c_numeric_operand_production() {
    let registry = CommandRegistry::build_default();
    let context = crate::model::ingress::static_context_for("jim");
    let dialect = InvocationDialect::of_profile(context.commands().profile().unwrap());
    let values = [InvocationWord::Literal("a b"), InvocationWord::Dynamic];
    let words = InvocationWords::structured(InvocationWord::Literal("lindex"), &values)
        .with_dialect(dialect);
    let facts = registry
        .resolve_structured_invocation(words, dialect.authoring_query())
        .resolved()
        .unwrap()
        .facts();
    assert!(
        facts
            .integer_index_operand_conversion(
                words.arguments(),
                NativeCompilationSelection::Generic,
                1,
                NativeCompilationWordShape::Substituted
            )
            .is_none()
    );
}

#[test]
fn native_operand_cache_fixtures_preserve_original_object_and_fallback_distinctions() {
    let mut count = 0;
    for (profile, fixture) in PROFILES.iter().zip(FIXTURES) {
        for line in fixture.lines() {
            let mode = field(line, "mode");
            let case = field(line, "case");
            let integer_only = matches!(case, "0" | "1" | "2" | "4");
            if mode == "3" {
                assert_eq!(field(line, "cache"), "list", "{profile}: {line}");
            } else if mode == "4" && field(line, "code") == "0" && integer_only {
                assert_eq!(field(line, "cache"), "double", "{profile}: {line}");
            } else if matches!(mode, "0" | "1" | "2") && field(line, "code") == "0" && integer_only
            {
                if mode == "0" && *profile == "tcl8.4" && case == "4" {
                    assert_eq!(field(line, "cache"), "string");
                } else {
                    assert!(
                        matches!(field(line, "cache"), "int" | "wideInt" | "bignum"),
                        "{profile}: {line}"
                    );
                }
            }
            count += 1;
        }
    }
    assert_eq!(count, 210);
}

#[test]
fn native_get_number_original_cache_matrix_preserves_double_and_string_fallback() {
    let fixtures = [
        include_str!(
            "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.4.20.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.5.19.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.6.18.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.0.4.txt"
        ),
        include_str!(
            "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.1.0.txt"
        ),
    ];
    let mut count = 0;
    for (profile, fixture) in PROFILES.iter().zip(fixtures) {
        for line in fixture.lines() {
            assert_eq!(field(line, "code"), "0", "{profile}: {line}");
            match field(line, "mode") {
                "0" | "2" | "3" => {
                    assert_eq!(field(line, "after"), "int", "{profile}: {line}");
                }
                "1" => {
                    assert!(matches!(field(line, "after"), "int" | "wideInt"));
                }
                "4" => {
                    assert_eq!(field(line, "before"), field(line, "after"));
                    assert_eq!(
                        field(line, "after"),
                        if *profile == "tcl8.4" { "list" } else { "dict" }
                    );
                }
                "5" => assert_eq!(field(line, "after"), "double"),
                _ => panic!("unexpected fixture: {line}"),
            }
            count += 1;
        }
    }
    assert_eq!(count, 30);
}

#[test]
fn stock_integer_contents_recipe_matches_public_object_lineages() {
    let rows = [
        include_str!("../../tests/data/native_stock_integer_lineage/8.4.tsv"),
        include_str!("../../tests/data/native_stock_integer_lineage/8.5.tsv"),
        include_str!("../../tests/data/native_stock_integer_lineage/8.6.tsv"),
        include_str!("../../tests/data/native_stock_integer_lineage/9.0.tsv"),
        include_str!("../../tests/data/native_stock_integer_lineage/9.1.tsv"),
    ];
    for (version, rows) in VERSIONS.into_iter().zip(rows) {
        assert_eq!(rows.lines().count(), 24);
        let recipe =
            NativeStockIntegerContentsProtocol::select(InvocationDialect::for_version(version));
        assert_eq!(recipe.is_some(), version != TclVersion::V8_4);
        let Some(recipe) = recipe else {
            continue;
        };
        for value in ["0", "1", "-0", "18446744073709551616"] {
            assert!(recipe.accepts_integer_contents(value));
        }
        for value in ["1.0", "Inf", "NaN", "1\0tail", "abc"] {
            assert!(!recipe.accepts_integer_contents(value));
        }
        for row in rows.lines().filter(|row| row.starts_with("raw")) {
            assert!(
                row.split('\t').any(|field| field == "code=0"),
                "{version:?}: {row}"
            );
        }
        let constructor = rows
            .lines()
            .find(|row| row.starts_with("double1-constructor\tincr\t"))
            .unwrap();
        assert!(constructor.contains("\tbefore:double:0:"));
        assert!(constructor.contains("\tcode=1\t"));
        assert!(constructor.ends_with("312e3022"));
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert!(NativeStockIntegerContentsProtocol::select(jim).is_none());
}
