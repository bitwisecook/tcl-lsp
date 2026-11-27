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

//! Grammars of the compiled `string` members, `subst` templates, `regexp` and `regsub`.

use super::implementation_lookup::NativeCompilerImplementationLookup;
use super::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
    NativeNamedInvocationProtocol, literal_shape,
};
use crate::{InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;
use tcl_native_model::InvocationDialect;

// TclCompileSubstCmd (C8.6–9.1): TclWordKnownAtCompileTime options are
// considered only when the final template is the original SIMPLE_WORD.
pub(super) fn substitution_template_compiler_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    use NativeCompilationWordShape as Shape;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    match shapes.last() {
        None | Some(Shape::Substituted | Shape::BackslashLiteral | Shape::Expanded) => {
            return Selection::Generic;
        }
        Some(Shape::Opaque) => return Selection::Unknown,
        Some(_) => {}
    }
    for shape in &shapes[..shapes.len() - 1] {
        if matches!(shape, Shape::Substituted | Shape::Expanded) {
            return Selection::Generic;
        }
        if *shape == Shape::Opaque {
            return Selection::Unknown;
        }
    }
    let arguments = words.arguments();
    let kinds = match crate::substitution::compiler_template_kinds(arguments, version) {
        Ok(kinds) => kinds,
        Err(selection) => return selection,
    };
    let Some(template) = arguments.literal_at(shapes.len() - 1) else {
        return Selection::Unknown;
    };
    let dialect = InvocationDialect::for_version(version);
    let flags = tcl_lexer::word_parts::SubstFlags {
        vars: kinds.variables,
        cmds: kinds.commands,
        backslashes: kinds.backslashes,
        ..Default::default()
    };
    let Ok(end) = u32::try_from(template.len()) else {
        return Selection::Unknown;
    };
    let Ok(arena) = tcl_lexer::word_parts::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(template.as_bytes()),
        tcl_lexer::Span::new(0, end),
        flags,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    ) else {
        return Selection::Unknown;
    };
    // TclSubstCompile emits text, scalar reads and, each in its own catch
    // range, command substitutions and array reads with a command index. A
    // template TclSubstParse rejects part-way is not modelled.
    if arena.all_parts().any(|part| {
        matches!(
            part.part,
            tcl_lexer::word_parts::ExecutablePart::ParseError(_)
                | tcl_lexer::word_parts::ExecutablePart::Expression { .. }
        )
    }) {
        return Selection::Unknown;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(super) fn string_match_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
    scope: crate::native_string_compilation::NativeStringMatchScope,
) -> NativeCompilationSelection {
    let from = usize::from(
        scope == crate::native_string_compilation::NativeStringMatchScope::PublicMember,
    );
    if from == 1
        && !shapes.first().is_some_and(|shape| {
            literal_shape(*shape)
                && (version != TclVersion::V8_4 || *shape == NativeCompilationWordShape::Literal)
        })
    {
        return NativeCompilationSelection::Generic;
    }
    let Some(arguments) = shapes.get(from..) else {
        return NativeCompilationSelection::Generic;
    };
    crate::native_string_compilation::select(
        arguments,
        words.arguments().literal_at(from).map(str::as_bytes),
        version,
        operation,
        from,
    )
}

/// Metadata view of original named-compiler operands, without registration proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedCompilerOperandView {
    /// The supplied slice begins at the actual selected worker's operands.
    SelectedWorker,
    /// Registry facts have projected synthetic members out of these arguments.
    OriginalArguments {
        /// Original selector words still retained before the worker operands.
        selectors_retained: usize,
    },
    /// No producer has retained the original operand layout.
    Unknown,
}

/// The operand count `version`'s member compiler accepts.
#[must_use]
pub fn named_arity_at(
    arity: crate::Arity,
    later_arity: Option<(TclVersion, crate::Arity)>,
    version: TclVersion,
) -> crate::Arity {
    match later_arity {
        Some((from, later)) if version >= from => later,
        _ => arity,
    }
}

pub(super) fn named_ensemble_grammar(
    lookup: &'static NativeCompilerImplementationLookup,
    hook_from: TclVersion,
    arity: crate::Arity,
    view: NamedCompilerOperandView,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> NativeCompilationSelection {
    let from = match view {
        NamedCompilerOperandView::SelectedWorker => 0,
        NamedCompilerOperandView::OriginalArguments { selectors_retained } => selectors_retained,
        NamedCompilerOperandView::Unknown => {
            return if version < hook_from {
                NativeCompilationSelection::Generic
            } else {
                NativeCompilationSelection::Unknown
            };
        }
    };
    if version < hook_from
        || shapes.len() < from
        || !shapes[..from].iter().all(|shape| literal_shape(*shape))
        || !u16::try_from(shapes.len() - from).is_ok_and(|count| arity.accepts(count))
    {
        return NativeCompilationSelection::Generic;
    }
    NativeCompilationSelection::NamedInvocation {
        lookup,
        arguments_from: from,
        protocol: NativeNamedInvocationProtocol::Direct,
    }
}

pub(super) const STRING_EQUAL_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "equal",
        slot: "::tcl::string::equal",
        command: "string",
        prepended: &["equal"],
    };

pub(super) const STRING_LENGTH_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "length",
        slot: "::tcl::string::length",
        command: "string",
        prepended: &["length"],
    };

pub(super) fn string_trim_grammar(
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
    scope: crate::native_scalar_compilation::NativeScalarScope,
) -> NativeCompilationSelection {
    let from =
        usize::from(scope == crate::native_scalar_compilation::NativeScalarScope::PublicMember);
    if version < TclVersion::V8_6
        || !shapes.get(from..).is_some_and(|arguments| {
            matches!(arguments.len(), 1 | 2)
                && !arguments.contains(&NativeCompilationWordShape::Expanded)
        })
    {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    }
}

pub(super) fn string_member_arity_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
    lookup: &'static NativeCompilerImplementationLookup,
    scope: crate::native_scalar_compilation::NativeScalarScope,
    arity: usize,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let private = scope == crate::native_scalar_compilation::NativeScalarScope::PrivateOperands;
    let from = usize::from(!private);
    if !private {
        let Some(shape) = shapes.first() else {
            return Selection::Generic;
        };
        // C84 parses the raw member token, including delimiters. Later
        // ensemble compilers select a literal member's unwrapped value.
        if (version == TclVersion::V8_4 && *shape != NativeCompilationWordShape::Literal)
            || !literal_shape(*shape)
            || !words
                .arguments()
                .literal_at(0)
                .is_some_and(|member| !member.is_empty() && lookup.member.starts_with(member))
        {
            return Selection::Generic;
        }
    } else if version == TclVersion::V8_4 {
        return Selection::Generic;
    }
    if shapes.len().checked_sub(from) == Some(arity) {
        return Selection::Inline {
            operation,
            guard: NativeCompilationGuard::for_c_version(version),
        };
    }
    if !private && version >= TclVersion::V8_6 {
        Selection::NamedInvocation {
            lookup,
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    } else {
        Selection::Generic
    }
}

// TclCompileRegexpCmd in all pinned C releases accepts only -nocase
// abbreviations and -- before exactly two matching operands. Captures and
// other options invoke the live handler; see `native_regexp_compilation`.
pub(super) fn regexp_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let index = match regexp_compiler_pattern_index(words, shapes) {
        Ok(index) => index,
        Err(selection) => return selection,
    };
    if !crate::native_regexp_compilation::modelled(version) {
        return Selection::Unknown;
    }
    let saw_last = index
        .checked_sub(1)
        .and_then(|last| words.arguments().literal_at(last))
        == Some("--");
    // An analysis shape keeps a bare `a$` Literal; C splits it (see
    // `splits_into_text_tokens`), so it is no SIMPLE pattern here either.
    let pattern = crate::native_regexp_compilation::simple_word_bytes(
        shapes[index],
        words.arguments().literal_at(index).map(str::as_bytes),
    )
    .filter(|pattern| {
        shapes[index] == NativeCompilationWordShape::BracedLiteral
            || !splits_into_text_tokens(pattern)
    });
    if literal_shape(shapes[index]) && words.arguments().literal_at(index).is_none() {
        Selection::Unknown
    } else if version == TclVersion::V8_4 {
        use crate::native_regexp_compilation::{C84Pattern, c84_pattern};
        match pattern
            .filter(|pattern| pattern.first() != Some(&b'-'))
            .map(c84_pattern)
        {
            Some(C84Pattern::Plain(..) | C84Pattern::Empty) => Selection::Inline {
                operation,
                guard: NativeCompilationGuard::for_c_version(version),
            },
            Some(C84Pattern::Declined) | None => Selection::Generic,
        }
    } else if pattern.is_some_and(|pattern| pattern.first() == Some(&b'-')) && !saw_last {
        Selection::Generic
    } else {
        Selection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    }
}

// TclCompileRegsubCmd (C8.6-9.1) emits INST_STR_MAP only for
// `regsub -all ?--? simpleRE string simpleReplacement`: a compile-time pattern
// whose TclReToGlob form is `*text*` with no other metacharacters and is
// neither exact nor quantified, and a compile-time replacement without `\` or
// `&`. Every other form returns TCL_ERROR and invokes the command. That one
// accepted form's emission is unmodelled.
pub(super) fn regsub_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    use NativeCompilationWordShape as Shape;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    if shapes.contains(&Shape::Opaque) {
        return Selection::Unknown;
    }
    if shapes.contains(&Shape::Expanded) || !(4..=5).contains(&shapes.len()) {
        return Selection::Generic;
    }
    // A word known at compile time; `Err` carries the selection when it is not.
    let known = |index: usize| -> Result<&str, Selection> {
        match shapes[index] {
            Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral => words
                .arguments()
                .literal_at(index)
                .ok_or(Selection::Unknown),
            Shape::BackslashLiteral => Err(Selection::Unknown),
            _ => Err(Selection::Generic),
        }
    };
    let select = || -> Result<Selection, Selection> {
        if crate::native_regexp_compilation::simple_word_bytes(
            shapes[0],
            words.arguments().literal_at(0).map(str::as_bytes),
        ) != Some(b"-all".as_slice())
        {
            return Ok(Selection::Generic);
        }
        let mut pattern_index = 1;
        let mut pattern = known(pattern_index)?;
        if pattern.starts_with('-') {
            if pattern != "--" || shapes.len() == 4 {
                return Ok(Selection::Generic);
            }
            pattern_index = 2;
            pattern = known(pattern_index)?;
        } else if shapes.len() == 5 {
            return Ok(Selection::Generic);
        }
        let replacement = known(pattern_index + 2)?;
        Ok(
            if regsub_simple_glob(pattern.as_bytes())
                && !replacement.bytes().any(|byte| matches!(byte, b'\\' | b'&'))
            {
                Selection::Unknown
            } else {
                Selection::Generic
            },
        )
    };
    select().unwrap_or_else(|selection| selection)
}

/// `TclReToGlob` succeeds without exact or quantified matching and yields
/// `*text*` whose non-empty text has no glob metacharacter.
fn regsub_simple_glob(pattern: &[u8]) -> bool {
    let quantified = !pattern.starts_with(b"***=") && {
        let mut index = 0;
        let mut found = false;
        while index < pattern.len() {
            match pattern[index] {
                b'\\' => index += 1,
                b'.' if matches!(pattern.get(index + 1), Some(b'*' | b'+')) => {
                    found = true;
                    break;
                }
                _ => {}
            }
            index += 1;
        }
        found
    };
    let Some((glob, exact)) = tcl_syntax::glob::re_to_glob(pattern) else {
        return false;
    };
    if exact || quantified {
        return false;
    }
    match glob.as_slice() {
        [b'*', text @ .., b'*'] => {
            !text.is_empty()
                && !text
                    .iter()
                    .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'\\'))
        }
        _ => false,
    }
}

pub(super) fn regexp_compiler_pattern_index(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
) -> Result<usize, NativeCompilationSelection> {
    use crate::native_regexp_compilation::{RegexpOption, regexp_option, simple_word_bytes};
    use NativeCompilationSelection as Selection;
    if shapes.len() < 2 {
        return Err(Selection::Generic);
    }
    let mut index = 0;
    while index < shapes.len() - 2 {
        if !literal_shape(shapes[index]) {
            return Err(Selection::Generic);
        }
        let Some(option) = words.arguments().literal_at(index) else {
            return Err(Selection::Unknown);
        };
        index += 1;
        match simple_word_bytes(shapes[index - 1], Some(option.as_bytes())).and_then(regexp_option)
        {
            Some(RegexpOption::Last) => break,
            Some(RegexpOption::Nocase) => {}
            None => return Err(Selection::Generic),
        }
    }
    if shapes.len() - index == 2 {
        Ok(index)
    } else {
        Err(Selection::Generic)
    }
}

/// `ParseTokens` gives a `$` that starts no variable a TEXT token of its
/// own, so an unbraced word holding one beside other text has several
/// components and is not a `SIMPLE_WORD`.
#[must_use]
pub fn splits_into_text_tokens(value: &[u8]) -> bool {
    value.len() > 1 && value.contains(&b'$')
}
