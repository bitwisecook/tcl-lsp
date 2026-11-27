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

//! The command and variable names a compiler resolves while it prepares its original arguments, and when tracked names survive it.

use super::implementation_lookup::NativeCompilerImplementationLookup;
use super::string_grammars::named_arity_at;
use super::{
    NativeCompilationContext, NativeCompilationGrammar, NativeCompilationSelection,
    NativeCompilationSpec, NativeNamedInvocationProtocol, literal_shape,
};
use crate::SemanticOperationId;
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;
use tcl_native_model::InvocationDialect;

/// Effects of a selected original compiler before its arguments execute.
/// This supplies no opcode admission, native cache or normal handler result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOriginalCompilerNameEffects {
    /// The selected compiler prepares data without tracked name mutations.
    Preserved,
    /// A fresh command-name literal requires an independent quiet exact lookup.
    FixedLookup(&'static NativeCompilerImplementationLookup),
    /// Quiet original command-literal lookup followed by ordered word preparation.
    FixedLookupThenVisits {
        /// The independently selected private compiler command.
        lookup: &'static NativeCompilerImplementationLookup,
        /// Existing original-word compiler effects after that lookup.
        visits: Vec<crate::native_control_compilation::NativeControlPreparationStep>,
    },
    /// Ordered preparation visits whose nested compiler effects remain separate.
    Visits(Vec<crate::native_control_compilation::NativeControlPreparationStep>),
    /// Compiler callbacks or nested source effects remain unresolved.
    Unknown,
}

impl NativeCompilationSpec {
    /// Name effects with an explicit unresolved command-literal lookup. Source
    /// consumers may close that lookup only in their independently retained
    /// resolver/table world; a closed native table alone supplies no resolver.
    #[must_use]
    pub fn original_argument_name_effects(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
    ) -> NativeOriginalCompilerNameEffects {
        use NativeOriginalCompilerNameEffects::{FixedLookup, Preserved, Unknown};
        let Some(dialect) = dialect else {
            return Unknown;
        };
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if dialect.family() != Some(Family::Tcl)
            || words.source_protocol()
                != tcl_syntax::native_string::NativeStringProtocol::C(version)
            || operand_from == 0
            || operand_from > words.original_words().len()
            || !self.operation_matches_grammar()
        {
            return Unknown;
        }
        if self.compiler_hook_presence(dialect) == Some(false) {
            return Preserved;
        }
        if let NativeCompilationGrammar::StringMap(scope) = self.grammar {
            return Self::original_string_map_name_effects(words, operand_from, scope, dialect);
        }
        if self.preserves_names_before_original_arguments(words, operand_from, Some(dialect)) {
            return Preserved;
        }
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true)
            || !(0..words.original_words().len()).all(|index| words.literal(index).is_some())
        {
            return Unknown;
        }
        match self.grammar {
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.original_argument_name_effects(words, operand_from, Some(dialect))
            }
            NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup,
                implementation_from,
                hook_from,
                arity,
                later_arity,
            } if version >= implementation_from && version >= hook_from => {
                let arity = named_arity_at(arity, later_arity, version);
                let count = words.original_words().len() - operand_from;
                if u16::try_from(count).is_ok_and(|count| arity.accepts(count)) {
                    FixedLookup(lookup)
                } else {
                    // The BasicNArg wrapper declines before command literal preparation.
                    Preserved
                }
            }
            _ => Unknown,
        }
    }

    fn original_string_map_name_effects(
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        scope: crate::native_scalar_compilation::NativeScalarScope,
        dialect: InvocationDialect,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand::Original;
        use crate::native_control_compilation::NativeControlPreparationStep::Word;
        use NativeOriginalCompilerNameEffects::{
            FixedLookup, FixedLookupThenVisits, Unknown, Visits,
        };
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true) {
            return Unknown;
        }
        let from = operand_from
            + usize::from(
                scope == crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            );
        let lookup = &crate::native_string_map_compilation::STRING_MAP_IMPLEMENTATION;
        match crate::native_string_map_compilation::select_original(
            words,
            operand_from,
            scope,
            version,
        ) {
            NativeCompilationSelection::NamedInvocation {
                protocol: NativeNamedInvocationProtocol::Direct,
                ..
            } => FixedLookupThenVisits {
                lookup,
                visits: (from..words.original_words().len())
                    .map(|index| Word(Original(index)))
                    .collect(),
            },
            // INVOKE_REPLACE prepares the words before its command literal.
            NativeCompilationSelection::NamedInvocation { .. }
                if (0..words.original_words().len())
                    .all(|index| words.literal(index).is_some()) =>
            {
                FixedLookup(lookup)
            }
            NativeCompilationSelection::Inline { .. } => Visits(vec![Word(Original(from + 1))]),
            NativeCompilationSelection::Generic => {
                Visits(Self::original_generic_word_visits(words))
            }
            _ => Unknown,
        }
    }

    fn original_list_replace_name_effects(
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_control_compilation::NativeControlPreparationStep::Word;
        use NativeOriginalCompilerNameEffects::{Unknown, Visits};
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true) {
            return Unknown;
        }
        if let Some(recipe) = crate::native_list_replace_compilation::compile_native_list_replace(
            words,
            operand_from,
            version,
        ) {
            return Visits(recipe.compiled_words().into_iter().map(Word).collect());
        }
        if crate::native_list_replace_compilation::select_original(words, operand_from, version)
            == NativeCompilationSelection::Generic
        {
            Visits(Self::original_generic_word_visits(words))
        } else {
            Unknown
        }
    }

    fn original_string_replace_name_effects(
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        scope: crate::native_scalar_compilation::NativeScalarScope,
        dialect: InvocationDialect,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_control_compilation::NativeControlPreparationStep::Word;
        use NativeOriginalCompilerNameEffects::{FixedLookup, Unknown, Visits};
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true) {
            return Unknown;
        }
        if let Some(recipe) =
            crate::native_string_replace_compilation::compile_native_string_replace(
                words,
                operand_from,
                scope,
                version,
            )
        {
            return Visits(recipe.compiled_words().into_iter().map(Word).collect());
        }
        match crate::native_string_replace_compilation::select_original(
            words,
            operand_from,
            scope,
            version,
        ) {
            NativeCompilationSelection::Generic => {
                Visits(Self::original_generic_word_visits(words))
            }
            // INVOKE_REPLACE prepares the words before its command literal.
            NativeCompilationSelection::NamedInvocation { .. }
                if (0..words.original_words().len())
                    .all(|index| words.literal(index).is_some()) =>
            {
                FixedLookup(
                    &crate::native_string_replace_compilation::STRING_REPLACE_IMPLEMENTATION,
                )
            }
            _ => Unknown,
        }
    }

    fn original_string_insert_name_effects(
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        scope: crate::native_scalar_compilation::NativeScalarScope,
        dialect: InvocationDialect,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_control_compilation::NativeControlPreparationStep::Word;
        use NativeOriginalCompilerNameEffects::{FixedLookup, Unknown, Visits};
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true) {
            return Unknown;
        }
        if let Some(recipe) = crate::native_string_insert_compilation::compile_native_string_insert(
            words,
            operand_from,
            scope,
            version,
        ) {
            return Visits(recipe.compiled_words().into_iter().map(Word).collect());
        }
        match crate::native_string_insert_compilation::select_original(
            words,
            operand_from,
            scope,
            version,
        ) {
            NativeCompilationSelection::Generic => {
                Visits(Self::original_generic_word_visits(words))
            }
            NativeCompilationSelection::NamedInvocation { .. }
                if (0..words.original_words().len())
                    .all(|index| words.literal(index).is_some()) =>
            {
                FixedLookup(&crate::native_string_insert_compilation::STRING_INSERT_IMPLEMENTATION)
            }
            _ => Unknown,
        }
    }

    fn original_generic_word_visits(
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    ) -> Vec<crate::native_control_compilation::NativeControlPreparationStep> {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand;
        use crate::native_control_compilation::NativeControlPreparationStep as Visit;
        (0..words.original_words().len())
            .map(|index| Visit::Word(NativeCompilerWordOperand::Original(index)))
            .collect()
    }

    /// Project preparation effects without changing the selected execution mode
    /// or issuing an inline instruction. Generic word compilation and nested
    /// body visits retain their independent source and lookup obligations.
    #[must_use]
    pub fn original_argument_name_effects_in_context(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeOriginalCompilerNameEffects {
        use NativeOriginalCompilerNameEffects::{Preserved, Unknown, Visits};
        let effects = self.original_argument_name_effects(words, operand_from, dialect);
        let Some(dialect) = dialect else {
            return Unknown;
        };
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if dialect.family() != Some(Family::Tcl)
            || words.source_protocol()
                != tcl_syntax::native_string::NativeStringProtocol::C(version)
            || operand_from == 0
            || operand_from > words.original_words().len()
            || !self.operation_matches_grammar()
        {
            return Unknown;
        }
        if effects != Unknown {
            // A null command compileProc still leaves the enclosing generic
            // script compiler responsible for bracketed word substitutions.
            return if effects == Preserved
                && self.compiler_hook_presence(dialect) == Some(false)
                && (0..words.original_words().len()).any(|index| words.literal(index).is_none())
            {
                Visits(Self::original_generic_word_visits(words))
            } else {
                effects
            };
        }
        if let NativeCompilationGrammar::StringReplace(scope) = self.grammar {
            return Self::original_string_replace_name_effects(words, operand_from, scope, dialect);
        }
        if let NativeCompilationGrammar::StringInsert(scope) = self.grammar {
            return Self::original_string_insert_name_effects(words, operand_from, scope, dialect);
        }
        if self.grammar == NativeCompilationGrammar::ListReplacement {
            return Self::original_list_replace_name_effects(words, operand_from, dialect);
        }
        if let NativeCompilationGrammar::StringRange(scope) = self.grammar {
            return self.original_string_range_name_effects(
                words,
                operand_from,
                scope,
                dialect,
                context,
            );
        }
        self.selected_original_argument_name_effects(words, operand_from, dialect, version, context)
    }

    fn original_string_range_name_effects(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        scope: crate::native_scalar_compilation::NativeScalarScope,
        dialect: InvocationDialect,
        context: NativeCompilationContext,
    ) -> NativeOriginalCompilerNameEffects {
        use NativeOriginalCompilerNameEffects::{Unknown, Visits};
        let Some(version) = dialect.tcl_version else {
            return Unknown;
        };
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true)
            || self.compiler_hook_presence(dialect) != Some(true)
            || !matches!(
                crate::native_string_range_compilation::select_original(
                    words,
                    operand_from,
                    scope,
                    version,
                    self,
                    context
                ),
                NativeCompilationSelection::Inline { .. }
            )
        {
            return Unknown;
        }
        crate::native_string_range_compilation::compile_native_string_range(
            words,
            operand_from,
            scope,
            version,
        )
        .map_or(Unknown, |recipe| {
            use crate::native_control_compilation::NativeControlPreparationStep::Word;
            let mut visits = vec![Word(recipe.operand)];
            if let crate::native_string_range_compilation::NativeStringRangeOperation::Indices {
                first,
                last,
            } = recipe.operation
            {
                visits.extend([Word(first), Word(last)]);
            }
            Visits(visits)
        })
    }

    fn selected_original_argument_name_effects(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand;
        use crate::native_control_compilation::NativeControlPreparationStep as Visit;
        use NativeOriginalCompilerNameEffects::{Unknown, Visits};
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true)
            || self.compiler_hook_presence(dialect) != Some(true)
        {
            return Unknown;
        }
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            return compiler.original_argument_name_effects_in_context(
                words,
                operand_from,
                Some(dialect),
                context,
            );
        }
        if let NativeCompilationGrammar::StringIsAscii(scope) = self.grammar
            && crate::native_string_is_compilation::private_original_declines(
                words,
                operand_from,
                scope,
                version,
                self.operation,
                self.body,
                context,
            )
        {
            return Visits(Self::original_generic_word_visits(words));
        }
        if self.grammar == NativeCompilationGrammar::FormatStringConcat {
            if crate::native_format_compilation::dynamic_format_word_declines(
                words,
                operand_from,
                version,
                context,
                self.operation,
                self.body,
            ) {
                return Visits(Self::original_generic_word_visits(words));
            }
            let selection = crate::native_format_compilation::select_with_values(
                &|index| words.literal(operand_from + index).map(<[u8]>::to_vec),
                &words.shapes()[operand_from..],
                version,
                context,
                self.operation,
            );
            return match selection {
                NativeCompilationSelection::Inline { .. } => Visits(
                    ((operand_from + 1)..words.original_words().len())
                        .map(|index| Visit::Word(NativeCompilerWordOperand::Original(index)))
                        .collect(),
                ),
                NativeCompilationSelection::Generic => {
                    Visits(Self::original_generic_word_visits(words))
                }
                _ => Unknown,
            };
        }
        // These selected leaf emitters prepare source tokens, not variable
        // values. Substitution compiler visits remain explicitly unresolved.
        if self.preserves_names_for_literal_vector(words, operand_from) {
            return Visits(Self::original_generic_word_visits(words));
        }
        if self.grammar == NativeCompilationGrammar::Expression {
            let operand = NativeCompilerWordOperand::Original(operand_from);
            return if words.original_words().len() == operand_from + 1
                && crate::native_expression_program::prepare_native_expression_program(
                    words, &operand, dialect,
                )
                .is_ok()
            {
                Visits(vec![Visit::Expression(operand)])
            } else {
                Unknown
            };
        }
        self.selected_control_name_effects(words, operand_from, dialect, context)
    }

    fn selected_control_name_effects(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
        context: NativeCompilationContext,
    ) -> NativeOriginalCompilerNameEffects {
        use crate::native_control_compilation::NativeControlOutcome;
        use NativeOriginalCompilerNameEffects::{Unknown, Visits};
        let control = match self.grammar {
            NativeCompilationGrammar::Conditional
            | NativeCompilationGrammar::WhileLoop
            | NativeCompilationGrammar::ForLoop
            | NativeCompilationGrammar::Catch => {
                crate::native_control_instructions::native_control_instruction(
                    self.grammar,
                    words,
                    operand_from,
                    dialect,
                    context,
                )
                .map(|control| {
                    (
                        control.preparations,
                        matches!(control.outcome, NativeControlOutcome::Generic),
                    )
                })
            }
            _ => return Unknown,
        };
        let Ok((mut visits, generic)) = control else {
            return Unknown;
        };
        if generic {
            visits.extend(Self::original_generic_word_visits(words));
        }
        Visits(visits)
    }

    /// Whether the selected compiler can preserve tracked command and
    /// namespace bindings before this original vector's argv evaluation.
    /// This is independent of opcode admission, variable storage, literal
    /// caches and handler completion. Unrepresented nested compilation or
    /// implementation paths return `false`.
    #[must_use]
    pub fn preserves_names_before_original_arguments(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
    ) -> bool {
        let Some(dialect) = dialect else {
            return false;
        };
        let Some(version) = dialect.tcl_version else {
            return false;
        };
        if dialect.family() != Some(Family::Tcl)
            || words.source_protocol()
                != tcl_syntax::native_string::NativeStringProtocol::C(version)
            || operand_from == 0
            || operand_from > words.original_words().len()
            || !self.operation_matches_grammar()
        {
            return false;
        }
        if self.grammar == NativeCompilationGrammar::NoHook {
            return true;
        }
        if words.shapes()[0].compiler_head(Some(dialect)) != Some(true)
            || !(0..words.original_words().len()).all(|index| words.literal(index).is_some())
        {
            return false;
        }
        self.preserves_names_for_literal_vector(words, operand_from)
    }

    // One arm per grammar: the table is exhaustive by design.
    #[allow(clippy::too_many_lines)]
    fn preserves_names_for_literal_vector(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
    ) -> bool {
        let count = words.original_words().len() - operand_from;
        match self.grammar {
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.operation_matches_grammar()
                    && compiler.preserves_names_for_literal_vector(words, operand_from)
            }
            NativeCompilationGrammar::VariableLoadStore => {
                self.operation
                    == SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::Set)
                    && matches!(count, 1 | 2)
            }
            NativeCompilationGrammar::StringEqual(_) => count == 2,
            NativeCompilationGrammar::StringIsAscii(scope) => {
                string_is_ascii_preserves_names(words, operand_from, scope)
            }
            NativeCompilationGrammar::StringLength(_)
            | NativeCompilationGrammar::ListLength
            | NativeCompilationGrammar::NamespaceOrigin
            | NativeCompilationGrammar::NamespaceString(_) => count == 1,
            NativeCompilationGrammar::StringOperand { operation, .. } => operation.accepts(count),
            NativeCompilationGrammar::StringMatch(_) => {
                count == 2 || (count == 3 && words.literal(operand_from) == Some(b"-nocase"))
            }
            NativeCompilationGrammar::StringTrim { .. } => matches!(count, 1 | 2),
            NativeCompilationGrammar::NamespaceCurrent => count == 0,
            NativeCompilationGrammar::NamespaceWhich => {
                namespace_which_preserves_names(words, operand_from)
            }
            NativeCompilationGrammar::NamespaceCode => {
                count == 1
                    && literal_shape(words.shapes()[operand_from])
                    && !words
                        .literal(operand_from)
                        .is_some_and(|value| value.starts_with(b"::namespace inscope "))
            }
            NativeCompilationGrammar::InfoCommands => {
                info_commands_preserves_names(words, operand_from, count)
            }
            NativeCompilationGrammar::ArityFrom { .. } => {
                self.operation == SemanticOperationId::Intrinsic(crate::IntrinsicId::ListSet)
            }
            NativeCompilationGrammar::Dictionary { command, .. } => {
                dictionary_preserves_names(command)
            }
            NativeCompilationGrammar::NoHook
            | NativeCompilationGrammar::VariableAppend(_)
            | NativeCompilationGrammar::MathOperator(_)
            | NativeCompilationGrammar::ClockReading(_)
            | NativeCompilationGrammar::TclOoHelper(_)
            | NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::NamespaceLegacy
            | NativeCompilationGrammar::NamespaceUpvarBindings
            | NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings
            | NativeCompilationGrammar::Upvar
            | NativeCompilationGrammar::Uplevel
            | NativeCompilationGrammar::LiteralUnset
            | NativeCompilationGrammar::InfoExists
            | NativeCompilationGrammar::InfoLevel
            | NativeCompilationGrammar::ListIndex
            | NativeCompilationGrammar::ListRange
            | NativeCompilationGrammar::ListAssignment
            | NativeCompilationGrammar::ListInsertion
            | NativeCompilationGrammar::Error
            | NativeCompilationGrammar::Throw
            | NativeCompilationGrammar::Constant
            | NativeCompilationGrammar::Return
            | NativeCompilationGrammar::ArgumentList
            | NativeCompilationGrammar::ArgumentListFrom(_)
            | NativeCompilationGrammar::ArgumentConcatFrom(_)
            | NativeCompilationGrammar::CoroutineRelay
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::InfoCoroutine
            | NativeCompilationGrammar::Regexp
            | NativeCompilationGrammar::Increment
            | NativeCompilationGrammar::LoopControl
            | NativeCompilationGrammar::Break
            | NativeCompilationGrammar::Continue => true,
            // These paths can perform nested compilation, resolver-mediated
            // command literal preparation, or an unauthored compiler hook.
            NativeCompilationGrammar::Unresolved
            | NativeCompilationGrammar::FormatStringConcat
            | NativeCompilationGrammar::StringMap(_)
            | NativeCompilationGrammar::StringRange(_)
            | NativeCompilationGrammar::StringReplace(_)
            | NativeCompilationGrammar::StringInsert(_)
            | NativeCompilationGrammar::ListReplacement
            | NativeCompilationGrammar::HookFrom(_)
            | NativeCompilationGrammar::Regsub
            | NativeCompilationGrammar::ProcedureHookFrom(_)
            | NativeCompilationGrammar::NamedEnsembleInvocation { .. }
            | NativeCompilationGrammar::Array { .. }
            | NativeCompilationGrammar::CheckedArity { .. }
            | NativeCompilationGrammar::Conditional
            | NativeCompilationGrammar::Switch
            | NativeCompilationGrammar::Expression
            | NativeCompilationGrammar::SubstitutionTemplate
            | NativeCompilationGrammar::ForLoop
            | NativeCompilationGrammar::WhileLoop
            | NativeCompilationGrammar::Foreach
            | NativeCompilationGrammar::Catch
            | NativeCompilationGrammar::Try
            | NativeCompilationGrammar::LiteralOperands { .. } => false,
        }
    }
}

/// Whether a C `namespace which` operand shape compiles from its literal words.
fn namespace_which_preserves_names(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
) -> bool {
    let tcl_syntax::native_string::NativeStringProtocol::C(version) = words.source_protocol()
    else {
        return false;
    };
    crate::native_introspection_compilation::namespace_which_shape(
        &words.shapes()[operand_from..],
        words.literal(operand_from),
        version,
    ) != crate::native_introspection_compilation::NativeNamespaceWhichShape::Unavailable
}

fn string_is_ascii_preserves_names(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
    scope: crate::native_scalar_compilation::NativeScalarScope,
) -> bool {
    let from = operand_from
        + usize::from(scope == crate::native_scalar_compilation::NativeScalarScope::PublicMember);
    words.original_words().len().checked_sub(from) == Some(2)
        && words.literal(from) == Some(b"ascii")
}

fn info_commands_preserves_names(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
    count: usize,
) -> bool {
    count == 1
        && words.literal(operand_from).is_some_and(|value| {
            tcl_syntax::naming::is_absolute_bytes(value)
                && value
                    .iter()
                    .all(|&byte| byte.is_ascii() && !matches!(byte, 0 | b'*' | b'?' | b'[' | b'\\'))
        })
}

fn dictionary_preserves_names(command: crate::native_dictionary::NativeDictionaryCommand) -> bool {
    use crate::native_dictionary::NativeDictionaryCommand as Dictionary;
    matches!(
        command,
        Dictionary::Append
            | Dictionary::Create
            | Dictionary::Exists
            | Dictionary::Get
            | Dictionary::GetDefault
            | Dictionary::GetWithDefault
            | Dictionary::Incr
            | Dictionary::Lappend
            | Dictionary::Merge
            | Dictionary::Remove
            | Dictionary::Replace
            | Dictionary::Set
            | Dictionary::Unset
    )
}
