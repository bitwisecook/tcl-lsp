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

//! `string` — perform one of several string operations.

use crate::prelude::*;

mod constant_folds;
mod is_classes;

pub(crate) use constant_folds::{
    fold_cat, fold_compare, fold_equal, fold_first, fold_index, fold_last, fold_length, fold_match,
    fold_range, fold_range_unanimous, fold_repeat, fold_replace, fold_reverse, fold_string_map,
    fold_tolower, fold_totitle, fold_toupper, fold_trim, fold_trimleft, fold_trimright,
};
use is_classes::IS_CLASSES;
pub(crate) use is_classes::fold_is;
pub use is_classes::{string_is_member_type, string_is_member_type_for_dialect};

/// `string match ?-nocase? pattern string` — the declared option prefix shifts
/// the pattern.  The shared descriptor walk also accepts Tcl's unambiguous
/// `-noc` abbreviation without teaching any consumer about this subcommand.
fn match_arg_roles(
    args: crate::InvocationArguments<'_>,
    options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let count = args.exact_argv_len()?;
    let index = options.leading_word_count(args)?;
    Some(if index < count {
        vec![(u8::try_from(index).ok()?, ArgRole::Pattern)]
    } else {
        Vec::new()
    })
}

const MATCH_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-nocase",
    value: OptionValue::flag(),
    detail: "Match without regard to case; the endpoints of a [x-y] range are lower-cased first, so [A-z] behaves like [A-Za-z].",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "string option arg ?arg ...?",
    ..FormSpec::DEFAULT
}];

const OK_COMPLETION_CODES: &[CompletionCode] = &[CompletionCode::Ok];

use tcl_dialect::model::SpecSurface;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bytelength",
        // C86 TclCompileBasic1ArgCmd invokes the private worker by its full name.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "bytelength",
                    slot: "::tcl::string::bytelength",
                    command: "string",
                    prepended: &["bytelength"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                hook_from: tcl_dialect::TclVersion::V8_6,
                arity: Arity::exact(1),
                later_arity: None,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::exact(1),
        detail: "Return number of bytes used to represent the string in memory (obsolete; removed in Tcl 9.0 — use `string length` or `encoding convertto`).",
        synopsis: "string bytelength string",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "bytelength",
                    slot: "::tcl::string::bytelength",
                    command: "string",
                    prepended: &["bytelength"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        // Documented "likely to go away in a future release" as far back
        // as the Tcl 8.4 manpage, but only actually removed at the 9.0
        // boundary: present (still obsolete-tagged) in the 8.4, 8.5, and
        // 8.6 manpages; absent entirely from the 9.0 and 9.1 subcommand
        // lists. `TCL8X` is the exact 8.4|8.5|8.6 set — not a
        // closest-sounding stand-in.
        surface: Some(SpecSurface::TCL8X),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cat",
        // S110: conservative may-corrupt. When **every** operand is a pure
        // byte array the result stays a byte array in both 8.6 (`StringCatCmd`
        // → `Tcl_AppendObjToObj`'s bytearray fast path; tclsh 8.6.14-verified)
        // and 9.0 (`TclStringCat`'s pure-bytearray path) — but any operand
        // with a string rep (the typical `string cat $bytes " text"`) makes
        // the result a character string, and the pass cannot prove runtime
        // purity of every operand, so it warns on may-coerce.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::any(),
        detail: "Concatenate strings.",
        synopsis: "string cat ?string1? ?string2 ...?",
        // C 8.6+ TclCompileStringCatCmd (`strcat`) takes any operand count;
        // an expanded word leaves the command to run.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::Cat,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "cat",
                    slot: "::tcl::string::cat",
                    command: "string",
                    prepended: &["cat"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_cat),
        surface: Some(SpecSurface::TCL86_PLUS),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "compare",
        // C84 TclCompileStringCmd; C85+ TclCompileStringCmpCmd (`strcmp`), with
        // C86+ TclCompileEnsemble's private-name fallback on refused forms.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::Compare,
            },
            operation: SemanticOperationId::Intrinsic(IntrinsicId::StringCompare),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringCompare)),
        arity: Arity::at_least(2),
        reserved_trailing_words: 2,
        detail: "Compare two strings lexicographically.",
        synopsis: "string compare ?-nocase? ?-length length? string1 string2",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "compare",
                    slot: "::tcl::string::compare",
                    command: "string",
                    prepended: &["compare"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_compare),
        options: const {
            &[
                OptionSpec {
                    name: "-nocase",
                    value: OptionValue::flag(),
                    detail: "Compare without regard to case.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "-length",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Any),
                        hint: "length",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Compare only the first length characters; a negative value is ignored (the whole string is compared).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "equal",
        // C84 TclCompileStringCmd; C85+ TclCompileStringEqualCmd, with
        // C86+ TclCompileEnsemble's private-name fallback on refused forms.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Intrinsic(IntrinsicId::StringEqual),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringEqual)),
        arity: Arity::at_least(2),
        reserved_trailing_words: 2,
        detail: "Test string equality.",
        synopsis: "string equal ?-nocase? ?-length length? string1 string2",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "equal",
                    slot: "::tcl::string::equal",
                    command: "string",
                    prepended: &["equal"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Boolean),
        const_fold: Some(fold_equal),
        options: const {
            &[
                OptionSpec {
                    name: "-nocase",
                    value: OptionValue::flag(),
                    detail: "Compare without regard to case.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "-length",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Any),
                        hint: "length",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Compare only the first length characters; a negative value is ignored (the whole string is compared).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "first",
        // C86+ TclCompileStringFirstCmd (`strfind`); C84/C85 run the command.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::First,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::new(2, 3),
        detail: "Find first occurrence of needle in haystack.",
        synopsis: "string first needleString haystackString ?startIndex?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "first",
                    slot: "::tcl::string::first",
                    command: "string",
                    prepended: &["first"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_first),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STRING_FIRST),
        arg_types: &[
            (
                0,
                // Needle: 8.6 reads it via `Tcl_GetUnicodeFromObj`
                // (`StringFirstCmd`), installing the string intrep —
                // tclsh8.6-verified: a list needle becomes `string`
                // (`string first $l hay` → representation flips list→string).
                // 9.0's `TclStringFirst` keeps the rep only when needle AND
                // haystack are both pure byte arrays; 8.6 converts byte
                // arrays unconditionally. A positional hint cannot see the
                // sibling operand, so ByteArray is listed transparent — the
                // 9.0-safe under-approximation (silent on the 8.6
                // byte-array conversion rather than false-positive on 9.0's
                // both-byte-array fast path).
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                // Haystack: same `Tcl_GetUnicodeFromObj` read as the needle
                // (tclsh8.6-verified: `string first pat $l` flips the
                // haystack list→string) and the same 8.6/9.0 byte-array
                // split — see the needle's note.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "index",
        // C84 TclCompileStringCmd; C85+ TclCompileStringIndexCmd (`strindex`), with
        // C86+ TclCompileEnsemble's private-name fallback on refused forms.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::Index,
            },
            operation: SemanticOperationId::Intrinsic(IntrinsicId::StringIndex),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringIndex)),
        // S110: keeps a pure byte-array rep in both 8.6 and 9.0
        // (`StringIndexCmd` returns a one-byte `Tcl_NewByteArrayObj` on the
        // `TclIsPureByteArray` path; tclsh 8.6.14-verified).
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(2),
        detail: "Return character at index.",
        synopsis: "string index string charIndex",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "index",
                    slot: "::tcl::string::index",
                    command: "string",
                    prepended: &["index"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_index),
        arg_types: &[
            (
                0,
                // The subject installs the `tclStringType` intrep
                // (`SetStringFromAny` via the char-indexing path), replacing
                // a List/Dict/Int/… rep — tclsh-verified (a list becomes
                // `string`). A pure byte array short-circuits before the
                // conversion and keeps its rep.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "insert",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringInsert(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        source_index_bounds: Some(crate::SourceIndexBounds::StringInsert),
        // S110 (9.0-only subcommand): built on `TclStringReplace`, so a pure
        // byte-array operand with a pure byte-array insertion keeps the
        // byte-array rep, but the typical string insertion coerces —
        // `Coerces` is the may-corrupt classification.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::exact(3),
        detail: "Insert string at index.",
        synopsis: "string insert string index insertString",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "insert",
                    slot: "::tcl::string::insert",
                    command: "string",
                    prepended: &["insert"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL90_PLUS),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "is",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringIsAscii(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Intrinsic(IntrinsicId::StringIs),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringIs)),
        // `-failindex` is written only when the class test fails.
        traits: Traits::CONDITIONAL_VARIABLE_WRITE,
        arity: Arity::at_least(2),
        detail: "Test if string is a member of a character class.",
        synopsis: "string is class ?-strict? ?-failindex varname? string",
        return_type: Some(TclType::Boolean),
        // `TclCompileStringIsCmd` arrives in 8.6; before it the command runs.
        inline_codegen_hook_windows: &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("8.6"),
            value: crate::hooks::InlineCodegenHookId::StringIs,
        }],
        const_fold_versioned: Some(fold_is),
        subcommand_forms: const {
            &[crate::forms::SubCommandForm {
                name: "value",
                arity: Arity::exact(2),
                side_effects: Some(crate::knowledge::Knowledge::Partial(&[])),
                world_effects: crate::knowledge::Knowledge::Partial(
                    crate::WorldEffectDescriptor::EMPTY,
                ),
                state_transitions: Some(crate::StateTransitionDescriptor::EMPTY),
                result_stability: crate::knowledge::Knowledge::Partial(
                    crate::ResultStability::ReferentiallyTransparent,
                ),
                traits: Some(Traits::PURE.union(Traits::CSE_CANDIDATE)),
                ..crate::forms::SubCommandForm::DEFAULT
            }]
        },
        // Deliberately NOT `pure: true`: `-failindex` (documented since
        // Tcl 8.4) optionally writes its `varname` argument, and the
        // static spec can't see whether a given call uses it — the same
        // shape as `encoding convertfrom`/`convertto`'s `-failindex`,
        // resolved there the same way (see `commands/tcl/encoding_.rs`).
        // `classify_side_effects` (`tcl-compiler/src/side_effects.rs`)
        // trusts a subcommand's `pure` flag unconditionally and never even
        // looks at `side_effects` when it's set, so marking this pure
        // would let dead-code-elimination/CSE silently drop or reorder a
        // real variable write.
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        // `string is class ?-strict? ?-failindex varName? string`: the class
        // precedes the option run and the tested string follows it.
        option_prefix_words: 1,
        reserved_trailing_words: 1,
        options: const {
            &[
                OptionSpec {
                    name: "-strict",
                    value: OptionValue::flag(),
                    detail: "Treat the empty string as not matching the class (by default the empty string matches every class).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "-failindex",
                    value: OptionValue::Takes(OptionArg {
                        role: ArgRole::VarWrite,
                        hint: "varname",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Variable to receive the index where the class test failed. Left unset when string matches the class; on failure its exact contents are class-specific (e.g. -1 for a numeric overflow, the parse-failure index for list/dict, always 0 for boolean/true/false).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        // First sub-arg (index 0 after `is`) is the character
        // class — complete it from the fixed class set, which is
        // exhaustive (a non-member is a runtime `bad class` error → W127).
        // C Tcl accepts a unique prefix (`boo` → `boolean`).
        arg_values: &[(0, IS_CLASSES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "last",
        // C86+ TclCompileStringLastCmd (`strrfind`); C84/C85 run the command.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::Last,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::new(2, 3),
        detail: "Find last occurrence of needle in haystack.",
        synopsis: "string last needleString haystackString ?lastIndex?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "last",
                    slot: "::tcl::string::last",
                    command: "string",
                    prepended: &["last"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_last),
        arg_types: &[
            (
                0,
                // Needle: mirrors `string first` — 8.6's `StringLastCmd`
                // reads both operands via `Tcl_GetUnicodeFromObj`
                // (tclsh8.6-verified: a list needle flips to `string`);
                // 9.0's `TclStringLast` keeps the rep only for the
                // both-pure-byte-array pair, so ByteArray is listed
                // transparent as the 9.0-safe under-approximation.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                // Haystack: same read and same 8.6/9.0 byte-array split as
                // the needle (tclsh8.6-verified: `string last pat $l` flips
                // the haystack list→string).
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "length",
        // C84's monolithic TclCompileStringCmd; C85+ TclCompileStringLenCmd.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringLength)),
        // An already-resolved core `string length` does not consult TclOO or
        // `unknown`. Live binding, namespace lookup, execution traces, and
        // interpreter/runtime-version policy remain irreducible BASE guards.
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        arity: Arity::exact(1),
        completion: crate::knowledge::Knowledge::Partial(CompletionDescriptor::exact(
            OK_COMPLETION_CODES,
        )),
        detail: "Return number of characters.",
        synopsis: "string length string",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "length",
                    slot: "::tcl::string::length",
                    command: "string",
                    prepended: &["length"],
                },
            },
        ),
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_length),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STRING_LENGTH),
        // `Tcl_GetCharLength` installs the `tclStringType` intrep
        // (`SetStringFromAny`), replacing a List/Dict/Int/… rep —
        // tclsh-verified (`set i 5; incr i; string length $i` leaves `i` a
        // `string`). A pure byte array short-circuits to its byte count and
        // keeps its rep, so it is transparent here even though the
        // subcommand is no byte-array value-transform
        // (`byte_array_effect: None` — the result is a count, not bytes).
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
                transparent_from: &[TclType::ByteArray],
            },
        )],
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "map",
        native_result: Some(crate::native_result::NativeResultContract::StockResult),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMap(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // S110: always builds a character string from the Unicode rep in both
        // 8.6 and 9.0 (`StringMapCmd`; tclsh 8.6.14-verified — a bytearray
        // operand yields a string). The canonical K22406348 corruption step.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::at_least(2),
        detail: "Map substrings via key-value pairs.",
        synopsis: "string map ?-nocase? mapping string",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "map",
                    slot: "::tcl::string::map",
                    command: "string",
                    prepended: &["map"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_string_map),
        // Claimed per call, not per command — `string map {a b} $x` launders
        // nothing, so the colour is earned from the mapping the call was
        // written with (`taint::mapping_deletes_crlf`). The
        // CR/LF-stripping mapping is the fix T101 / IRULE3003 document and the
        // one their quick fix writes, so the colour it earns has to be the
        // colour those sinks read (`CRLF_FREE`).
        taint_transform: Some(TaintColour::CRLF_FREE),
        taint_transform_when: Some(TaintTransformCondition::MappingDeletesCrlf),
        options: const {
            &[OptionSpec {
                name: "-nocase",
                value: OptionValue::flag(),
                detail: "Match keys without regard to case.",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
                effect: None,
            }]
        },
        arg_types: &[
            (
                0,
                // Mapping: both 8.6 and 9.0 (`StringMapCmd`) take the dict
                // iteration path only for a *pure* dict (`tclDictType` with
                // no string rep) and hand everything else to
                // `TclListObjGetElements`, which installs the LIST intrep —
                // tclsh8.6-verified: a plain-string mapping flips to
                // `list`, a list mapping stays `list`, a `dict create`
                // mapping stays `dict`. Not hinted `expected: Dict`: a list
                // mapping never converts to dict, only to list. Dict is
                // listed transparent for the pure-dict path; a dict that
                // has regenerated its string rep does re-parse as a list,
                // which this positional hint cannot see — deliberate
                // under-approximation.
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                    transparent_from: &[TclType::Dict],
                },
            ),
            (
                1,
                // Subject: read via `Tcl_GetUnicodeFromObj` unconditionally
                // in both 8.6 and 9.0 (`StringMapCmd`), installing the
                // string intrep — tclsh8.6-verified: a list subject flips
                // list→string and a pure byte-array subject flips
                // bytearray→string (no byte-array fast path in either
                // release), so nothing is transparent here.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "match",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMatch(
                crate::native_string_compilation::NativeStringMatchScope::PublicMember,
            ),
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::at_least(2),
        detail: "Test glob-style pattern match.",
        synopsis: "string match ?-nocase? pattern string",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "match",
                    slot: "::tcl::string::match",
                    command: "string",
                    prepended: &["match"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        inline_codegen_hook: Some(crate::hooks::InlineCodegenHookId::StringMatch),
        const_fold: Some(fold_match),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STRING_MATCH),
        return_type: Some(TclType::Boolean),
        options: MATCH_OPTIONS,
        arg_role_layout_resolver: Some(match_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Pattern],
        pattern_type: Some(PatternType::Glob),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "range",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringRange(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringRange)),
        // S110: keeps a pure byte-array rep in both 8.6 and 9.0
        // (`Tcl_GetRange`'s `TclIsPureByteArray` path returns
        // `Tcl_NewByteArrayObj`; tclsh 8.6.14-verified in the interpreted
        // and compiled paths).
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(3),
        detail: "Return substring by index range.",
        synopsis: "string range string first last",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "range",
                    slot: "::tcl::string::range",
                    command: "string",
                    prepended: &["range"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STRING_RANGE),
        const_fold: Some(fold_range_unanimous),
        const_fold_versioned: Some(fold_range),
        arg_types: &[
            (
                0,
                // Subject: installs the string intrep except for the pure
                // byte-array fast path — see `string index` arg 0.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "repeat",
        native_result: Some(crate::native_result::NativeResultContract::StockResult),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "repeat",
                    slot: "::tcl::string::repeat",
                    command: "string",
                    prepended: &["repeat"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                hook_from: tcl_dialect::TclVersion::V8_6,
                arity: Arity::exact(2),
                later_arity: None,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // S110: 8.6 builds the result from the string rep (`StringReptCmd` →
        // `TclGetStringFromObj` + fresh buffer; tclsh 8.6.14-verified — a
        // bytearray operand yields a pure string). 9.0 differs:
        // `TclStringRepeat` pre-sizes a pure byte-array result for a pure
        // byte-array operand. `Coerces` is the 8.6-conservative may-corrupt
        // classification.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::exact(2),
        detail: "Repeat string N times.",
        synopsis: "string repeat string count",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "repeat",
                    slot: "::tcl::string::repeat",
                    command: "string",
                    prepended: &["repeat"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_repeat),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "replace",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringReplace(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringReplace)),
        // S110: 8.6 always builds the result from the Unicode rep
        // (`StringRplcCmd` → `Tcl_GetUnicodeFromObj`; tclsh 8.6.14-verified —
        // a bytearray operand yields a string). 9.0 differs: `TclStringReplace`
        // keeps a pure byte-array rep when the operand *and* the replacement
        // are both pure byte arrays (or the replacement is omitted); a string
        // replacement still coerces. `Coerces` is the 8.6-conservative
        // may-corrupt classification.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::new(3, 4),
        detail: "Replace range with new string.",
        synopsis: "string replace string first last ?newString?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "replace",
                    slot: "::tcl::string::replace",
                    command: "string",
                    prepended: &["replace"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        // `TclCompileStringReplaceCmd` arrives in 8.6; before it the command
        // runs.
        inline_codegen_hook_windows: &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("8.6"),
            value: crate::hooks::InlineCodegenHookId::StringReplace,
        }],
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_replace),
        arg_types: &[
            (
                0,
                // Subject: installs the string intrep — tclsh8.6-verified:
                // a list subject flips list→string while a pure byte-array
                // subject keeps its rep (8.6's index computation
                // short-circuits on `TclIsPureByteArray`, and 9.0's
                // `TclStringReplace` has the same pure-byte-array path), so
                // ByteArray is transparent.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "reverse",
        // C86+ TclCompileBasic1ArgCmd invokes the private worker by its full name.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "reverse",
                    slot: "::tcl::string::reverse",
                    command: "string",
                    prepended: &["reverse"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                hook_from: tcl_dialect::TclVersion::V8_6,
                arity: Arity::exact(1),
                later_arity: None,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // S110: keeps a pure byte-array rep in both 8.6 and 9.0
        // (`TclStringReverse`'s `TclIsPureByteArray` path reverses the bytes;
        // tclsh 8.6.14-verified).
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(1),
        detail: "Reverse character order.",
        synopsis: "string reverse string",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "reverse",
                    slot: "::tcl::string::reverse",
                    command: "string",
                    prepended: &["reverse"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_reverse),
        // Added in Tcl 8.5.
        surface: Some(SpecSurface::TCL85_PLUS),
        arg_types: &[(
            0,
            // Subject: `TclStringReverse` calls `SetStringFromAny` in 8.6
            // (`Tcl_GetUnicode` in 9.0) for anything that is not a pure
            // byte array — tclsh8.6-verified: a list subject flips
            // list→string; a pure byte-array subject keeps its rep via the
            // `TclIsPureByteArray` byte-reversal path in both releases.
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
                transparent_from: &[TclType::ByteArray],
            },
        )],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "tolower",
        // C86+ TclCompileStringToLowerCmd (`strcaseLower`), else
        // TclCompileBasic1To3ArgCmd; C84/C85 run the command.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::ToLower,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        byte_array_effect: ByteArrayEffect::CaseFolds,
        arity: Arity::new(1, 3),
        detail: "Convert to lower case.",
        synopsis: "string tolower string ?first? ?last?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "tolower",
                    slot: "::tcl::string::tolower",
                    command: "string",
                    prepended: &["tolower"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_tolower),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "totitle",
        // C86+ TclCompileStringToTitleCmd (`strcaseTitle`), else
        // TclCompileBasic1To3ArgCmd; C84/C85 run the command.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::ToTitle,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        byte_array_effect: ByteArrayEffect::CaseFolds,
        arity: Arity::new(1, 3),
        detail: "Convert to title case.",
        synopsis: "string totitle string ?first? ?last?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "totitle",
                    slot: "::tcl::string::totitle",
                    command: "string",
                    prepended: &["totitle"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_totitle),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "toupper",
        // C86+ TclCompileStringToUpperCmd (`strcaseUpper`), else
        // TclCompileBasic1To3ArgCmd; C84/C85 run the command.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringOperand {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation:
                    crate::native_string_operand_compilation::NativeStringOperandOperation::ToUpper,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        byte_array_effect: ByteArrayEffect::CaseFolds,
        arity: Arity::new(1, 3),
        detail: "Convert to upper case.",
        synopsis: "string toupper string ?first? ?last?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "toupper",
                    slot: "::tcl::string::toupper",
                    command: "string",
                    prepended: &["toupper"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_toupper),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "trim",
        native_result: Some(crate::native_result::NativeResultContract::StockResult),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringTrim {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation: crate::native_string_trim_compilation::NativeStringTrimOperation::Both,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // Whenever `trim` actually strips characters it builds a fresh
        // *string* from the UTF rep in both 8.6 and 9.0 (`StringTrimCmd`
        // → `Tcl_NewStringObj`, and the compiled `INST_STR_TRIM`
        // likewise; tclsh 8.6.14-verified — trimming whitespace off a
        // bytearray yields a pure string). The byte-array rep survives
        // only the compiled *no-op* trim (nothing stripped — the same
        // object is returned) — 8.6's interpreted command proc
        // strings-ifies even that — so `Coerces` (may-corrupt) is the
        // honest classification for both versions.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::new(1, 2),
        detail: "Trim leading and trailing characters.",
        synopsis: "string trim string ?chars?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "trim",
                    slot: "::tcl::string::trim",
                    command: "string",
                    prepended: &["trim"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        // `TclCompileStringTrimCmd` arrives in 8.6; before it the command runs.
        inline_codegen_hook_windows: &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("8.6"),
            value: crate::hooks::InlineCodegenHookId::StringTrim,
        }],
        return_type: Some(TclType::String),
        const_fold: Some(fold_trim),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "trimleft",
        native_result: Some(crate::native_result::NativeResultContract::StockResult),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringTrim {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation: crate::native_string_trim_compilation::NativeStringTrimOperation::Left,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // An effective trim builds a fresh string in both 8.6 and 9.0
        // (`StringTrimLCmd` / `INST_STR_TRIM_LEFT`); see `trim` above.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::new(1, 2),
        detail: "Trim leading characters.",
        synopsis: "string trimleft string ?chars?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "trimleft",
                    slot: "::tcl::string::trimleft",
                    command: "string",
                    prepended: &["trimleft"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        // `TclCompileStringTrimLCmd` arrives in 8.6; before it the command runs.
        inline_codegen_hook_windows: &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("8.6"),
            value: crate::hooks::InlineCodegenHookId::StringTrimLeft,
        }],
        return_type: Some(TclType::String),
        const_fold: Some(fold_trimleft),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "trimright",
        native_result: Some(crate::native_result::NativeResultContract::StockResult),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringTrim {
                scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                operation: crate::native_string_trim_compilation::NativeStringTrimOperation::Right,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // An effective trim builds a fresh string in both 8.6 and 9.0
        // (`StringTrimRCmd` / `INST_STR_TRIM_RIGHT`); see `trim` above.
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::new(1, 2),
        detail: "Trim trailing characters.",
        synopsis: "string trimright string ?chars?",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "trimright",
                    slot: "::tcl::string::trimright",
                    command: "string",
                    prepended: &["trimright"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        // `TclCompileStringTrimRCmd` arrives in 8.6; before it the command runs.
        inline_codegen_hook_windows: &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("8.6"),
            value: crate::hooks::InlineCodegenHookId::StringTrimRight,
        }],
        return_type: Some(TclType::String),
        const_fold: Some(fold_trimright),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::CLOSED_REFERENTIALLY_TRANSPARENT
    },
    SubCommand {
        name: "wordend",
        // C86+ TclCompileBasic2ArgCmd invokes the private worker by its full name.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "wordend",
                    slot: "::tcl::string::wordend",
                    command: "string",
                    prepended: &["wordend"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                hook_from: tcl_dialect::TclVersion::V8_6,
                arity: Arity::exact(2),
                later_arity: None,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::exact(2),
        detail: "Index of character after end of word.",
        synopsis: "string wordend string charIndex",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "wordend",
                    slot: "::tcl::string::wordend",
                    command: "string",
                    prepended: &["wordend"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        // Subject (index 0) deliberately carries NO hint: 8.6's
        // `StringEndCmd` reads it via `TclGetStringFromObj` + `Tcl_UtfAtIndex`
        // (dual-ported — tclsh8.6-verified: a list subject stays `list`).
        // 9.0 switched to `Tcl_GetUnicodeFromObj`, which DOES install the
        // string intrep; the hint table is dialect-agnostic, so stamping it
        // would false-positive on 8.6 — the dominant dialect — and stays
        // un-stamped per the verified-only rule.
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "wordstart",
        // C86+ TclCompileBasic2ArgCmd invokes the private worker by its full name.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "wordstart",
                    slot: "::tcl::string::wordstart",
                    command: "string",
                    prepended: &["wordstart"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                hook_from: tcl_dialect::TclVersion::V8_6,
                arity: Arity::exact(2),
                later_arity: None,
            },
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        arity: Arity::exact(2),
        detail: "Index of first character of word.",
        synopsis: "string wordstart string charIndex",
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5String),
                implementation_from: tcl_dialect::TclVersion::V8_5,
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::string",
                    member: "wordstart",
                    slot: "::tcl::string::wordstart",
                    command: "string",
                    prepended: &["wordstart"],
                },
            },
        ),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        pure: true,
        return_type: Some(TclType::Int),
        // Subject (index 0) deliberately un-stamped — same 8.6 dual-ported
        // read / 9.0 `Tcl_GetUnicodeFromObj` divergence as `wordend`.
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `string`.
///
/// The ensemble's own dispatch synopsis (`string option arg ?arg ...?`) and
/// the bulk of its subcommands (`compare`/`equal`/`first`/`index`/`is`/
/// `last`/`length`/`map`/`match`/`range`/`repeat`/`replace`/`tolower`/
/// `totitle`/`toupper`/`trim`/`trimleft`/`trimright`/`wordend`/`wordstart`,
/// each with their documented options) are unchanged Tcl 8.4 → 9.1. The
/// subcommand *set* is not, though — each gate is on the matching
/// `SubCommand.surface` below, not here:
///
/// * `reverse` — added Tcl 8.5 (absent from the 8.4 manpage's subcommand
///   list; present from 8.5's).
/// * `cat` — added Tcl 8.6 (absent from 8.4 and 8.5).
/// * `bytelength` — documented "likely to go away in a future release" as
///   far back as 8.4, but only actually removed at the Tcl 9.0 boundary:
///   present in 8.4, 8.5, and 8.6; gone from the 9.0 and 9.1 manpages.
/// * `insert` — added Tcl 9.0 (absent from 8.4 through 8.6).
///
/// `string is`'s character-class set grows the same way — see the
/// `IS_CLASSES` table's individual `min_tcl` gates: `list` and
/// `wideinteger` (8.5), `entier` (8.6), `dict` (9.0 only). Every other
/// class has been present since 8.4.
///
/// The generic index syntax shared by most subcommands that take one
/// (`index`/`range`/`replace`, the `first`/`last` of `first`/`last`,
/// `tolower`/`toupper`/`totitle`'s `first`/`last`) also widened at 8.5:
/// the Tcl 8.4 manpage documents only a plain non-negative integer, `end`,
/// and `end-N`; Tcl 8.5 added a dedicated "STRING INDICES" section (which
/// 8.4's manpage has no equivalent of) documenting the further `end+N`,
/// `M+N`, and `M-N` arithmetic forms.
///
/// `trim`/`trimleft`/`trimright`'s default (no explicit `chars`) character
/// set also widened at 8.6: 8.4/8.5 document it as literally "spaces,
/// tabs, newlines, and carriage returns"; from 8.6 the manpage instead
/// defines it as "any character for which `string is space` returns 1,
/// and \0" — i.e. it additionally strips NUL, plus whatever `string is
/// space`'s own (also-growing) Unicode class covers. `TRIM_WS` below
/// still only encodes the narrower 8.4/8.5 set (see its own doc comment).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "string",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Unresolved,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("string"),
        // Present and unrestricted: its surface carries an iRules row
        // explicitly (`ALL_TCL.union(IRULES)`), so it resolves under the
        // iRules point — a pure value-transform ensemble with no
        // filesystem/process/network access, so every dialect that hosts a
        // real Tcl core (irules, iapps, tmsh, the EDA shells, expect, tk,
        // itcl) carries it unmodified. Individual subcommands and `string is`
        // classes still narrow per Tcl version through their own
        // `dialects`/`min_tcl` gates (enforced generically by the profile's
        // `version_ceiling`, the same mechanism that keeps `return`'s `-level`
        // out of iRules) — not a whole-command dialect gate.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Perform one of several string operations, selected by the first argument.",
            synopsis: &["string option arg ?arg ...?"],
            snippet: "Dispatches to a subcommand (length, match, is, compare, map, range, index, trim, ... — see completion for the full list); an unrecognised option is a runtime error, not a compile-time one, and an unambiguous prefix of a subcommand name is accepted. Nearly every subcommand is a pure value transform with no side effects; the one exception is `is`'s `-failindex varname`, which writes the class-test failure position into varname. Index arguments (string index/range/replace, the first/last of tolower/toupper/totitle, the startIndex/lastIndex of first/last) accept a plain non-negative integer, end (the last character), or end-N; Tcl 8.5+ also accepts end+N and the M+N/M-N arithmetic forms. The subcommand set itself has grown over releases: reverse and the is list/wideinteger classes need Tcl 8.5+; cat and the is entier class need Tcl 8.6+; insert and the is dict class need Tcl 9.0+; bytelength was removed at the Tcl 9.0 boundary after being documented obsolete since 8.4 — prefer string length (character count) or the encoding command (byte count) instead. trim/trimleft/trimright strip whitespace by default when chars is omitted; from Tcl 8.6 that default set also strips NUL (\\0), on top of the (also Unicode-aware and separately growing) whitespace string is space matches.",
            source: "Tcl string(n)",
            examples: "set s \"  Hello, World!  \"\nstring trim $s                   ;# -> \"Hello, World!\"\nstring tolower [string trim $s]  ;# -> \"hello, world!\"\nstring map {World Tcl} $s        ;# -> \"  Hello, Tcl!  \"\nstring range $s 2 6              ;# -> \"Hello\"\nstring is integer -strict 42     ;# -> 1\n\nif {[string first \"World\" $s] >= 0} {\n    puts \"found\"\n}",
            return_value: "Depends on the subcommand: most string-valued subcommands (cat, index, insert, map, range, replace, reverse, tolower/toupper/totitle, trim/trimleft/trimright) return a (possibly empty) string; is, equal, and match return a boolean 0/1; compare returns -1/0/1; length, wordend, wordstart, and bytelength (through Tcl 8.6) return a non-negative integer; first and last return a non-negative index, or -1 when there is no match.",
        }),
        inline_codegen_hook: Some(crate::hooks::InlineCodegenHookId::String),
        forms: FORMS,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests;
