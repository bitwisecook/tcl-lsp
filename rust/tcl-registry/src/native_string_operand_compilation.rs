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

//! Fixed-operand String member compilers and their actual registration scopes.
//!
//! C Tcl compiles these `string` members to one stack instruction over their
//! evaluated operands (`generic/tclCompCmdsSZ.c`; C 8.4's monolithic
//! `TclCompileStringCmd` in `generic/tclCompCmds.c`):
//!
//! | member | instruction | hook from | wrong operand count |
//! |---|---|---|---|
//! | `compare` | `strcmp` | 8.4 | `TCL_ERROR` |
//! | `index` | `strindex` | 8.4 | `TCL_ERROR` |
//! | `first` | `strfind` | 8.6 | `TCL_ERROR` |
//! | `last` | `strrfind` | 8.6 | `TCL_ERROR` |
//! | `toupper`, `tolower`, `totitle` | `strupper`, `strlower`, `strtitle` | 8.6 | `TclCompileBasic1To3ArgCmd` |
//! | `cat` | `strcat` (`str_concat1`) | 8.6 | any count: none pushes `""`, one is the word |
//!
//! `TclCompileStringCatCmd` folds each run of compile-known words into one
//! literal and issues `str_concat1` in chunks of at most 254 operands
//! (measured with `::tcl::unsupported::disassemble` on tclsh 8.6.18, 9.0.4
//! and 9.1.0; 8.5 has no `string cat`).
//!
//! A refused public ensemble member keeps its selected private worker through
//! the ensemble's invocation rewrite from C 8.6. `TclCompileBasic1To3ArgCmd`
//! instead invokes the worker's full name directly for one to three operands.
//! C 8.4 and 8.5 have no such fallback, and an earlier release without the hook
//! invokes the command generically.

use crate::SemanticOperationId;
use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;

/// Native String instruction over a fixed number of evaluated operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringOperandOperation {
    /// `string compare a b`: `strcmp`.
    Compare,
    /// `string index s i`: `strindex`.
    Index,
    /// `string first needle haystack`: `strfind`.
    First,
    /// `string last needle haystack`: `strrfind`.
    Last,
    /// `string toupper s`: `strupper`.
    ToUpper,
    /// `string tolower s`: `strlower`.
    ToLower,
    /// `string totitle s`: `strtitle`.
    ToTitle,
    /// `string cat ?s ...?`: `strcat` over the words, compile-known runs folded.
    Cat,
}

impl NativeStringOperandOperation {
    /// Every authored operation.
    pub const ALL: [Self; 8] = [
        Self::Compare,
        Self::Index,
        Self::First,
        Self::Last,
        Self::ToUpper,
        Self::ToLower,
        Self::ToTitle,
        Self::Cat,
    ];

    /// Whether the inline instruction takes `count` evaluated operands:
    /// two for the comparisons and searches, one for the case conversions,
    /// any number for `cat`.
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Self::Compare | Self::Index | Self::First | Self::Last => count == 2,
            Self::ToUpper | Self::ToLower | Self::ToTitle => count == 1,
            Self::Cat => true,
        }
    }

    /// First C release whose compiler selects this member's instruction.
    /// C 8.4's monolithic String compiler covers `compare` and `index` only.
    #[must_use]
    pub const fn hook_from(self) -> TclVersion {
        match self {
            Self::Compare | Self::Index => TclVersion::V8_4,
            Self::First
            | Self::Last
            | Self::ToUpper
            | Self::ToLower
            | Self::ToTitle
            | Self::Cat => TclVersion::V8_6,
        }
    }

    /// Whether a refused operand count falls through to
    /// `TclCompileBasic1To3ArgCmd`, a direct invocation of the worker.
    const fn basic_one_to_three(self) -> bool {
        matches!(self, Self::ToUpper | Self::ToLower | Self::ToTitle)
    }

    /// Actual private String worker registration (C 8.5 onwards).
    #[must_use]
    pub const fn implementation(self) -> &'static NativeCompilerImplementationLookup {
        match self {
            Self::Compare => &STRING_COMPARE_IMPLEMENTATION,
            Self::Index => &STRING_INDEX_IMPLEMENTATION,
            Self::First => &STRING_FIRST_IMPLEMENTATION,
            Self::Last => &STRING_LAST_IMPLEMENTATION,
            Self::ToUpper => &STRING_TOUPPER_IMPLEMENTATION,
            Self::ToLower => &STRING_TOLOWER_IMPLEMENTATION,
            Self::ToTitle => &STRING_TOTITLE_IMPLEMENTATION,
            Self::Cat => &STRING_CAT_IMPLEMENTATION,
        }
    }
}

macro_rules! string_worker {
    ($name:ident, $member:literal) => {
        #[doc = concat!("Genuine original private String `", $member, "` registration.")]
        pub const $name: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
            ensemble: "::string",
            member: $member,
            slot: concat!("::tcl::string::", $member),
            command: "string",
            prepended: &[$member],
        };
    };
}

string_worker!(STRING_COMPARE_IMPLEMENTATION, "compare");
string_worker!(STRING_INDEX_IMPLEMENTATION, "index");
string_worker!(STRING_FIRST_IMPLEMENTATION, "first");
string_worker!(STRING_LAST_IMPLEMENTATION, "last");
string_worker!(STRING_TOUPPER_IMPLEMENTATION, "toupper");
string_worker!(STRING_TOLOWER_IMPLEMENTATION, "tolower");
string_worker!(STRING_TOTITLE_IMPLEMENTATION, "totitle");
string_worker!(STRING_CAT_IMPLEMENTATION, "cat");

/// Chronological original operands followed by the selected instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeStringOperandInstruction {
    /// Selected native instruction.
    pub operation: NativeStringOperandOperation,
    /// Exact original words or original static expansion member receipts.
    pub operands: Vec<NativeCompilerWordOperand>,
    /// Each operand's compile-known value (`TclWordKnownAtCompileTime`);
    /// `cat` folds each run of them into one literal.
    pub literals: Vec<Option<Vec<u8>>>,
}

/// Select the actual compiler outcome from the post-head word shapes.
///
/// `shapes` starts at the public member word for [`NativeScalarScope::PublicMember`]
/// and at the first operand for a private worker; `member` is that public
/// member's literal value. Opaque operands retain an unknown outcome.
#[must_use]
pub fn select_shapes(
    shapes: &[Shape],
    member: Option<&[u8]>,
    operation: NativeStringOperandOperation,
    scope: NativeScalarScope,
    version: TclVersion,
    semantic: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let public = scope == NativeScalarScope::PublicMember;
    if version < operation.hook_from() || (!public && version == TclVersion::V8_4) {
        return Selection::Generic;
    }
    if public {
        let Some(shape) = shapes.first() else {
            return Selection::Generic;
        };
        // C84 parses the raw member token, including delimiters. Later
        // ensemble compilers select a literal member's unwrapped value.
        if (version == TclVersion::V8_4 && *shape != Shape::Literal)
            || !matches!(
                shape,
                Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
            )
            || !member.is_some_and(|member| {
                !member.is_empty()
                    && operation
                        .implementation()
                        .member
                        .as_bytes()
                        .starts_with(member)
            })
        {
            return Selection::Generic;
        }
    }
    let from = usize::from(public);
    let Some(operands) = shapes.get(from..) else {
        return Selection::Generic;
    };
    if operands.contains(&Shape::Opaque) {
        return Selection::Unknown;
    }
    // An expanded word bypasses every compiler hook.
    if operands.contains(&Shape::Expanded) {
        return Selection::Generic;
    }
    // TEMP measurement toggle (compiler-codegen), remove before hand-back.
    if operation == NativeStringOperandOperation::Cat
        && std::env::var_os("ZZ_CCG_BASELINE").is_some()
    {
        return Selection::Unknown;
    }
    if operation.accepts(operands.len()) {
        return Selection::Inline {
            operation: semantic,
            guard: NativeCompilationGuard::for_c_version(version),
        };
    }
    if operation.basic_one_to_three() && (1..=3).contains(&operands.len()) {
        return Selection::NamedInvocation {
            lookup: operation.implementation(),
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::Direct,
        };
    }
    if public && version >= TclVersion::V8_6 {
        return Selection::NamedInvocation {
            lookup: operation.implementation(),
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        };
    }
    Selection::Generic
}

/// Select the actual compiler's argument geometry over original words.
/// This grants no registration or native issuer authority; its caller
/// independently retains both.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    operation: NativeStringOperandOperation,
    scope: NativeScalarScope,
    version: TclVersion,
    semantic: SemanticOperationId,
) -> NativeCompilationSelection {
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return NativeCompilationSelection::Unknown;
    };
    let Some(arguments) = operand_from
        .checked_sub(1)
        .and_then(|_| projected.get(operand_from..))
    else {
        return NativeCompilationSelection::Unknown;
    };
    let shapes = arguments
        .iter()
        .map(|argument| argument.shape)
        .collect::<Vec<_>>();
    let member = arguments
        .first()
        .and_then(|argument| argument.literal.as_deref());
    select_shapes(&shapes, member, operation, scope, version, semantic)
}

/// Project only the independently admitted inline geometry onto original stack operands.
#[must_use]
pub fn compile_native_string_operand(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    operation: NativeStringOperandOperation,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringOperandInstruction> {
    if !matches!(
        select_original(
            words,
            operand_from,
            operation,
            scope,
            version,
            SemanticOperationId::Invoke
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let from = operand_from + usize::from(scope == NativeScalarScope::PublicMember);
    let operands = projected.get(from..)?;
    Some(NativeStringOperandInstruction {
        operation,
        operands: operands
            .iter()
            .map(|operand| operand.operand.clone())
            .collect(),
        literals: operands
            .iter()
            .map(|operand| operand.literal.clone())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use NativeStringOperandOperation as Operation;

    fn words(source: &str, version: TclVersion) -> Vec<Shape> {
        let dialect = crate::InvocationDialect::for_version(version);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let captured = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        captured.shapes().to_vec()
    }

    fn select(
        source: &str,
        operation: Operation,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        let dialect = crate::InvocationDialect::for_version(version);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let captured = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        select_original(
            &captured,
            1,
            operation,
            NativeScalarScope::PublicMember,
            version,
            SemanticOperationId::Invoke,
        )
    }

    /// Instruction windows measured with `::tcl::unsupported::disassemble
    /// script` on tclsh 8.4.20 (`tcl::unsupported` is absent there, so the
    /// 8.4 rows follow `TclCompileStringCmd`), 8.5.19, 8.6.18, 9.0.4 and
    /// 9.1.0: `strcmp`/`strindex` from 8.4, and `strfind`/`strrfind`/
    /// `strupper`/`strlower`/`strtitle` from 8.6.
    #[test]
    fn original_string_operand_windows_follow_each_release() {
        let inline = |selection| matches!(selection, NativeCompilationSelection::Inline { .. });
        for version in TclVersion::ALL {
            for (source, operation) in [
                ("string compare a b", Operation::Compare),
                ("string index $s 1", Operation::Index),
                ("string first a $s", Operation::First),
                ("string last a $s", Operation::Last),
                ("string toupper $s", Operation::ToUpper),
                ("string tolower $s", Operation::ToLower),
                ("string totitle $s", Operation::ToTitle),
            ] {
                assert_eq!(
                    inline(select(source, operation, version)),
                    version >= operation.hook_from(),
                    "{version:?}: {source}"
                );
            }
        }
        // An unknown or ambiguous member spelling never selects the hook here;
        // resolution of the public member belongs to the registry.
        assert_eq!(
            select("string x a b", Operation::Compare, TclVersion::V9_0),
            NativeCompilationSelection::Generic
        );
        // C84 reads the raw member token.
        assert_eq!(
            select("string {index} $s 1", Operation::Index, TclVersion::V8_4),
            NativeCompilationSelection::Generic
        );
        assert!(inline(select(
            "string {index} $s 1",
            Operation::Index,
            TclVersion::V8_5
        )));
    }

    /// `string cat` compiles from 8.6 at every operand count, as
    /// `::tcl::unsupported::disassemble` shows on tclsh 8.6.18, 9.0.4 and
    /// 9.1.0 (`push ""` for none, the word itself for one, `strcat` for
    /// more); an expanded word leaves the command to run, and each operand
    /// carries its compile-known value for the compiler's literal folding.
    #[test]
    fn string_cat_compiles_from_8_6_at_every_operand_count() {
        let inline = |selection| matches!(selection, NativeCompilationSelection::Inline { .. });
        for version in TclVersion::ALL {
            for source in ["string cat", "string cat $a", "string cat a - $b {c} d"] {
                assert_eq!(
                    inline(select(source, Operation::Cat, version)),
                    version >= TclVersion::V8_6,
                    "{version:?}: {source}"
                );
            }
        }
        assert_eq!(
            select("string cat a {*}$b", Operation::Cat, TclVersion::V9_0),
            NativeCompilationSelection::Generic
        );
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let source = "string cat a - $b {c} d";
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let captured = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let recipe = compile_native_string_operand(
            &captured,
            1,
            Operation::Cat,
            NativeScalarScope::PublicMember,
            TclVersion::V9_0,
        )
        .expect("inline");
        assert_eq!(recipe.operands.len(), 5);
        assert_eq!(
            recipe.literals,
            vec![
                Some(b"a".to_vec()),
                Some(b"-".to_vec()),
                None,
                Some(b"c".to_vec()),
                Some(b"d".to_vec()),
            ]
        );
    }

    #[test]
    fn refused_operand_counts_keep_the_actual_fallback() {
        use NativeNamedInvocationProtocol::{Direct, EnsembleRewrite};
        let named = |selection| match selection {
            NativeCompilationSelection::NamedInvocation { protocol, .. } => Some(protocol),
            _ => None,
        };
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                named(select("string index $s", Operation::Index, version)),
                Some(EnsembleRewrite)
            );
            assert_eq!(
                named(select("string toupper $s 1", Operation::ToUpper, version)),
                Some(Direct)
            );
            assert_eq!(
                named(select("string toupper $s 1 2", Operation::ToUpper, version)),
                Some(Direct)
            );
            assert_eq!(
                named(select("string toupper", Operation::ToUpper, version)),
                Some(EnsembleRewrite)
            );
            assert_eq!(
                named(select(
                    "string toupper a 1 2 3",
                    Operation::ToUpper,
                    version
                )),
                Some(EnsembleRewrite)
            );
            assert_eq!(
                select("string index {*}$l", Operation::Index, version),
                NativeCompilationSelection::Generic
            );
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                select("string index $s", Operation::Index, version),
                NativeCompilationSelection::Generic
            );
        }
        assert!(!words("string index $s 1", TclVersion::V9_0).is_empty());
    }

    #[test]
    fn private_workers_consume_only_their_operands() {
        for version in TclVersion::ALL {
            let selection = select_shapes(
                &[Shape::Literal, Shape::Literal],
                None,
                Operation::Index,
                NativeScalarScope::PrivateOperands,
                version,
                SemanticOperationId::Invoke,
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_5,
                "{version:?}"
            );
        }
        assert_eq!(
            select_shapes(
                &[Shape::Literal, Shape::Literal],
                None,
                Operation::ToUpper,
                NativeScalarScope::PrivateOperands,
                TclVersion::V9_0,
                SemanticOperationId::Invoke,
            ),
            NativeCompilationSelection::NamedInvocation {
                lookup: &STRING_TOUPPER_IMPLEMENTATION,
                arguments_from: 0,
                protocol: NativeNamedInvocationProtocol::Direct,
            }
        );
    }
}
