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

//! The `string equal`, `string length`, `regexp` and `regsub` compilers.

use super::*;

#[test]
fn string_equal_retains_raw_member_and_versioned_private_fallback() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::StringEqual(
            crate::native_scalar_compilation::NativeScalarScope::PublicMember,
        ),
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringEqual),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::QuotedLiteral,
            NativeCompilationWordShape::BracedLiteral,
        ] {
            let selection = spec.select(
                InvocationWords::literals("string", &["equal", "a", "a"]),
                &[
                    shape,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Substituted,
                ],
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal,
            );
        }
        let options = spec.select(
            InvocationWords::literals("string", &["equal", "-nocase", "a", "A"]),
            &[NativeCompilationWordShape::Literal; 4],
            Some(dialect),
            context(NativeCompilationFrame::ScriptCode),
        );
        assert_eq!(
            matches!(options, NativeCompilationSelection::NamedInvocation { .. }),
            version >= TclVersion::V8_6,
        );
        assert_eq!(
            spec.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_5
        );
    }
}

#[test]
fn string_length_requires_exact_member_shape_and_retains_native_arity_fallback() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::StringLength(
            crate::native_scalar_compilation::NativeScalarScope::PublicMember,
        ),
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringLength),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::QuotedLiteral,
            NativeCompilationWordShape::BracedLiteral,
        ] {
            let selected = spec.select(
                InvocationWords::literals("string", &["length", "value"]),
                &[shape, NativeCompilationWordShape::Substituted],
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal
            );
        }
        let selected = spec.select(
            InvocationWords::literals("string", &["length", "a", "b"]),
            &[NativeCompilationWordShape::Literal; 3],
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert_eq!(
            matches!(
                selected,
                NativeCompilationSelection::NamedInvocation {
                    protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                    ..
                }
            ),
            version >= TclVersion::V8_6
        );
        assert_eq!(
            spec.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_5
        );
    }
}

#[test]
fn regexp_capture_and_option_forms_decline_native_compilation() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    for version in TclVersion::ALL {
        for arguments in [
            &[][..],
            &["pattern"][..],
            &["pattern", "text", "capture"][..],
            &["-nocase", "pattern", "text", "capture"][..],
            &["--", "pattern", "text", "capture"][..],
            &["-expanded", "pattern", "text"][..],
            &["-start", "0", "pattern", "text"][..],
        ] {
            assert_eq!(
                native.select(
                    InvocationWords::literals("regexp", arguments),
                    &vec![NativeCompilationWordShape::Literal; arguments.len()],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                NativeCompilationSelection::Generic,
                "{version:?} {arguments:?}",
            );
        }
        for arguments in [
            &["pattern", "text"][..],
            &["-nocase", "pattern", "text"][..],
            &["-n", "--", "pattern", "text"][..],
        ] {
            assert_eq!(
                native.select(
                    InvocationWords::literals("regexp", arguments),
                    &vec![NativeCompilationWordShape::Literal; arguments.len()],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                regexp_selection(native, version),
                "{version:?} {arguments:?}",
            );
        }
    }
}

// tclsh 8.6.18/9.0.4 disassembly of `proc p {s} {...}`: every form below
// except the last pushes "regsub" and invokes it; `regsub -all {x} $s y`
// compiles to push "x"; push "y"; loadScalar; strmap, which stays unmodelled.
#[test]
fn regsub_invokes_every_form_its_compiler_declines() {
    use NativeCompilationWordShape::{BracedLiteral, Literal, Substituted};
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regsub").unwrap().native_compilation.unwrap();
    let select = |version, arguments: &[&str], shapes: &[NativeCompilationWordShape]| {
        native.select(
            InvocationWords::literals("regsub", arguments),
            shapes,
            Some(InvocationDialect::for_version(version)),
            context(NativeCompilationFrame::ProcedureCode),
        )
    };
    for version in TclVersion::ALL {
        for (arguments, shapes) in [
            (
                &["x", "$s", "y", "a"][..],
                &[BracedLiteral, Substituted, Literal, Literal][..],
            ),
            (
                &["-all", "x", "$s", "y", "a"][..],
                &[Literal, BracedLiteral, Substituted, Literal, Literal][..],
            ),
            (
                &["-all", "--", "x", "$s", "y", "a"][..],
                &[
                    Literal,
                    Literal,
                    BracedLiteral,
                    Substituted,
                    Literal,
                    Literal,
                ][..],
            ),
            (
                &["x", "$s", "y"][..],
                &[BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-nocase", "-all", "x", "$s", "y"][..],
                &[Literal, Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "a.c", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "^x$", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", ".*x", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "x", "$s", "&"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "$p", "$s", "y"][..],
                &[Literal, Substituted, Substituted, Literal][..],
            ),
        ] {
            assert_eq!(
                select(version, arguments, shapes),
                NativeCompilationSelection::Generic,
                "{version:?} {arguments:?}"
            );
        }
        let accepted = select(
            version,
            &["-all", "x", "$s", "y"],
            &[Literal, BracedLiteral, Substituted, Literal],
        );
        let expected = if version < TclVersion::V8_6 {
            NativeCompilationSelection::Generic
        } else {
            NativeCompilationSelection::Unknown
        };
        assert_eq!(accepted, expected, "{version:?}");
    }
}

// tclsh 9.0.4/9.1.0 `regexp $re $s`: loadScalar1; loadScalar1; regexp +3.
// The pre-9 regexp compilers are unmodelled.
/// A plain literal pattern: C8.4 compiles it under its chunk-entry guard,
/// C8.5 is unmodelled, and C8.6+ compiles it before its arguments.
fn regexp_selection(
    native: NativeCompilationSpec,
    version: TclVersion,
) -> NativeCompilationSelection {
    NativeCompilationSelection::Inline {
        operation: native.operation,
        guard: NativeCompilationGuard::for_c_version(version),
    }
}

#[test]
fn regexp_operand_layout_is_independent_of_unresolved_operation_selection() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    let words = InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &arguments);
    let shapes = [NativeCompilationWordShape::Substituted; 2];
    let compilation = context(NativeCompilationFrame::ProcedureCode);
    for version in TclVersion::ALL {
        let dialect = Some(InvocationDialect::for_version(version));
        // C8.4 compiles only a literal pattern; a substituted one declines.
        let expected = if version == TclVersion::V8_4 {
            NativeCompilationSelection::Generic
        } else {
            regexp_selection(native, version)
        };
        assert_eq!(
            native.select(words, &shapes, dialect, compilation),
            expected,
            "{version:?}"
        );
        assert_eq!(
            native.operand_layout(words, &shapes, dialect, compilation),
            (version >= TclVersion::V8_5).then_some(NativeCompilationOperandLayout::Pattern {
                argument_index: 0,
                guard: NativeCompilationGuard::BeforeArguments,
            })
        );
    }
    let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
    for compilation in [
        NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            ..compilation
        },
        NativeCompilationContext {
            frame: NativeCompilationFrame::Unknown,
            ..compilation
        },
    ] {
        assert_eq!(
            native.operand_layout(words, &shapes, dialect, compilation),
            None
        );
    }
    assert_eq!(
        native.operand_layout(words, &shapes, None, compilation),
        None
    );
    assert_eq!(
        native.operand_layout(
            words,
            &[
                NativeCompilationWordShape::Expanded,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        None
    );
}

#[test]
fn regexp_options_preserve_original_operand_layout() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
    let compilation = context(NativeCompilationFrame::ProcedureCode);
    let options = [
        crate::InvocationWord::Literal("-nocase"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &options),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        Some(NativeCompilationOperandLayout::Pattern {
            argument_index: 1,
            guard: NativeCompilationGuard::BeforeArguments
        })
    );
    let unknown_option = [crate::InvocationWord::Dynamic; 3];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &unknown_option),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation,
        ),
        None,
    );
    let generic = [
        crate::InvocationWord::Literal("-about"),
        crate::InvocationWord::Dynamic,
    ];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &generic),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        Some(NativeCompilationOperandLayout::Pattern {
            argument_index: 0,
            guard: NativeCompilationGuard::BeforeArguments
        })
    );
}
