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

//! Original native string-trim compiler geometry and default literal.

use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use crate::semantic_operation::SemanticOperationId;
use tcl_dialect::TclVersion;

/// Ends selected by the authenticated trim compiler registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeStringTrimOperation {
    /// Remove characters at both ends.
    Both,
    /// Remove leading characters.
    Left,
    /// Remove trailing characters.
    Right,
}

impl NativeStringTrimOperation {
    /// The private stock worker's original implementation path.
    #[must_use]
    pub const fn lookup(self) -> crate::native_compilation::NativeCompilerImplementationLookup {
        use crate::native_compilation::NativeCompilerImplementationLookup;
        match self {
            Self::Both => NativeCompilerImplementationLookup {
                ensemble: "::string",
                member: "trim",
                slot: "::tcl::string::trim",
                command: "string",
                prepended: &["trim"],
            },
            Self::Left => NativeCompilerImplementationLookup {
                ensemble: "::string",
                member: "trimleft",
                slot: "::tcl::string::trimleft",
                command: "string",
                prepended: &["trimleft"],
            },
            Self::Right => NativeCompilerImplementationLookup {
                ensemble: "::string",
                member: "trimright",
                slot: "::tcl::string::trimright",
                command: "string",
                prepended: &["trimright"],
            },
        }
    }
    /// Whether execution trims the left and right ends respectively.
    #[must_use]
    pub const fn ends(self) -> (bool, bool) {
        match self {
            Self::Both => (true, true),
            Self::Left => (true, false),
            Self::Right => (false, true),
        }
    }
}

/// Subject evaluation precedes the original character word or registered default.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeStringTrimInstruction {
    /// Original subject operand, without conversion during compilation.
    pub subject: NativeCompilerWordOperand,
    /// Original explicit trim-set operand; absence selects the default literal.
    pub characters: Option<NativeCompilerWordOperand>,
    /// Operation from the selected worker registration.
    pub operation: NativeStringTrimOperation,
}

/// Shared native trim default. Older commands use only these four ASCII bytes.
#[must_use]
pub fn default_trim_set(version: TclVersion) -> &'static [u8] {
    tcl_syntax::native_string_trim::default_trim_set(version)
}

/// Select the native arity and parser projection before operand evaluation.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let Ok(projected) = project_native_compiler_words(words, version) else {
        return Selection::Unknown;
    };
    let from = operand_from + usize::from(scope == NativeScalarScope::PublicMember);
    if scope == NativeScalarScope::PublicMember
        && !projected.get(operand_from).is_some_and(|word| {
            matches!(
                word.shape,
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
            )
        })
    {
        return Selection::Generic;
    }
    let Some(arguments) = projected.get(from..) else {
        return Selection::Unknown;
    };
    if arguments
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Opaque)
    {
        return Selection::Unknown;
    }
    if !matches!(arguments.len(), 1 | 2)
        || arguments
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

/// Retain original subject and character operands after an independent Inline selection.
#[must_use]
pub fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
    operation: NativeStringTrimOperation,
) -> Option<NativeStringTrimInstruction> {
    if !matches!(
        select_original(
            words,
            operand_from,
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
    Some(NativeStringTrimInstruction {
        subject: projected.get(from)?.operand.clone(),
        characters: projected.get(from + 1).map(|word| word.operand.clone()),
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    #[test]
    fn trim_compiler_geometry_matches_the_original_native_hook_boundaries() {
        for (version, engine) in [
            (TclVersion::V8_4, "tcl8.4"),
            (TclVersion::V8_5, "tcl8.5"),
            (TclVersion::V8_6, "tcl8.6"),
            (TclVersion::V9_0, "tcl9.0"),
            (TclVersion::V9_1, "tcl9.1"),
        ] {
            let environment = crate::model::ingress::resolve_environment(engine);
            let profile = environment.unit_profile();
            for operation in [
                NativeStringTrimOperation::Both,
                NativeStringTrimOperation::Left,
                NativeStringTrimOperation::Right,
            ] {
                for suffix in ["$s", "$s $c", "", "$s $c extra"] {
                    let source = format!("string {} {suffix}", operation.lookup().member);
                    let parsed = native_script_words_in(
                        SourceImage::native(source.as_bytes()),
                        Span::new(0, u32::try_from(source.len()).unwrap()),
                        LexerConfig::from_grammar(profile.grammar),
                    )
                    .unwrap();
                    let captured = NativeCompilerWords::capture(
                        &parsed.commands[0].words,
                        NativeStringProtocol::C(version),
                    )
                    .unwrap();
                    let selected = select_original(
                        &captured,
                        1,
                        NativeScalarScope::PublicMember,
                        version,
                        SemanticOperationId::Invoke,
                    );
                    let inline = version >= TclVersion::V8_6 && matches!(suffix, "$s" | "$s $c");
                    assert_eq!(
                        matches!(selected, NativeCompilationSelection::Inline { .. }),
                        inline,
                        "{version:?}/{source}"
                    );
                    let recipe = instruction(
                        &captured,
                        1,
                        NativeScalarScope::PublicMember,
                        version,
                        operation,
                    );
                    assert_eq!(recipe.is_some(), inline);
                    if let Some(recipe) = recipe {
                        assert_eq!(recipe.subject, NativeCompilerWordOperand::Original(2));
                        assert_eq!(
                            recipe.characters,
                            if suffix == "$s $c" {
                                Some(NativeCompilerWordOperand::Original(3))
                            } else {
                                None
                            }
                        );
                        assert_eq!(recipe.operation, operation);
                    }
                }
            }
        }
    }

    #[test]
    fn actual_trim_registration_keeps_jim_and_missing_engine_hooks_separate() {
        use crate::{InvocationDialect, model::ingress::resolve_environment};
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let environment = resolve_environment(engine);
            let profile = environment.unit_profile();
            let registry = crate::model::ingress::static_context_for_profile(profile).commands();
            let dialect = InvocationDialect::of_profile(profile);
            for operation in [
                NativeStringTrimOperation::Both,
                NativeStringTrimOperation::Left,
                NativeStringTrimOperation::Right,
            ] {
                let spec = registry
                    .native_compilation_for_registration(operation.lookup().slot, dialect)
                    .expect("actual stock private trim registration");
                assert_eq!(
                    spec.compiler_hook_presence(dialect),
                    Some(engine != "tcl8.5")
                );
                assert_eq!(
                    spec.compiler_hook_presence(InvocationDialect::of_profile(
                        resolve_environment("jim").unit_profile()
                    )),
                    Some(false)
                );
            }
        }
    }
}
