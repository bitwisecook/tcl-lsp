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

//! Lowering one word to a boxed value: which words are lowerable, variable reads and command substitutions.

use super::super::NativeLoweringDecline;
use super::super::cells::{CellPlace, VariableWordDecline, variable_word_place};
use super::super::ir::{NativeOp, NativeType, NativeValueId};
use super::super::representation::Representation;
use super::{Lowerer, MAX_WORD_DEPTH};
use crate::registry_invocation::{
    RegistryInvocationResolution, resolve_word_exprs,
    resolved_tokens_invocation_with_metadata_context,
};
use crate::types::TypeShape;
use tcl_registry::NativeLowering;
use tcl_registry::hooks::LoweringHookId;
use tcl_source::words::SourceSite;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

impl Lowerer<'_> {
    // Words.

    pub(super) fn words_lowerable(
        &self,
        words: &[WordExpr],
        depth: u32,
    ) -> Result<(), NativeLoweringDecline> {
        if depth > MAX_WORD_DEPTH {
            return Err(NativeLoweringDecline::WordNestingTooDeep);
        }
        if words.is_empty() {
            return Err(NativeLoweringDecline::MissingCommandTokens);
        }
        for word in words {
            self.word_lowerable(word, depth)?;
        }
        Ok(())
    }

    fn word_lowerable(&self, word: &WordExpr, depth: u32) -> Result<(), NativeLoweringDecline> {
        match word {
            WordExpr::Literal { .. } => Ok(()),
            WordExpr::BracedLiteral { text, .. } => {
                if text.contains('\\') {
                    Err(NativeLoweringDecline::BackslashSubstitution)
                } else {
                    Ok(())
                }
            }
            WordExpr::Variable { spelling, source } => {
                variable_place(spelling, source, self.lexer_config).map(|_| ())
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                let inner = nested_words(spelling, source, self.lexer_config)?;
                self.words_lowerable(&inner, depth + 1)
            }
            WordExpr::Template { parts, .. } => {
                if parts.is_empty() {
                    return Err(NativeLoweringDecline::OpaqueWord);
                }
                for part in parts {
                    match part {
                        WordPart::Text { text, .. } => {
                            if text.contains('\\') {
                                return Err(NativeLoweringDecline::BackslashSubstitution);
                            }
                        }
                        WordPart::Variable { spelling, source } => {
                            variable_place(spelling, source, self.lexer_config)?;
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            let inner = nested_words(spelling, source, self.lexer_config)?;
                            self.words_lowerable(&inner, depth + 1)?;
                        }
                        WordPart::Opaque { .. } => return Err(NativeLoweringDecline::OpaqueWord),
                    }
                }
                Ok(())
            }
            WordExpr::Expand { .. } => Err(NativeLoweringDecline::ArgumentExpansion),
            WordExpr::Opaque { .. } => Err(NativeLoweringDecline::OpaqueWord),
        }
    }

    /// Lower one word to a boxed value.
    pub(super) fn lower_word(
        &mut self,
        word: &WordExpr,
        depth: u32,
    ) -> Result<NativeValueId, NativeLoweringDecline> {
        if depth > MAX_WORD_DEPTH {
            return Err(NativeLoweringDecline::WordNestingTooDeep);
        }
        match word {
            WordExpr::Literal { text, .. } => Ok(self.const_str(text)),
            WordExpr::BracedLiteral { text, .. } => {
                if text.contains('\\') {
                    return Err(NativeLoweringDecline::BackslashSubstitution);
                }
                Ok(self.const_str(text))
            }
            WordExpr::Variable { spelling, source } => {
                let place = variable_place(spelling, source, self.lexer_config)?;
                let value = self.read_cell(&place);
                Ok(self.boxed(value))
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                self.lower_command_substitution(spelling, source, depth)
            }
            WordExpr::Template { parts, .. } => {
                if parts.is_empty() {
                    return Err(NativeLoweringDecline::OpaqueWord);
                }
                let mut values = Vec::with_capacity(parts.len());
                for part in parts {
                    values.push(match part {
                        WordPart::Text { text, .. } => {
                            if text.contains('\\') {
                                return Err(NativeLoweringDecline::BackslashSubstitution);
                            }
                            self.const_str(text)
                        }
                        WordPart::Variable { spelling, source } => {
                            let place = variable_place(spelling, source, self.lexer_config)?;
                            let value = self.read_cell(&place);
                            self.boxed(value)
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            self.lower_command_substitution(spelling, source, depth)?
                        }
                        WordPart::Opaque { .. } => return Err(NativeLoweringDecline::OpaqueWord),
                    });
                }
                if let [only] = values.as_slice() {
                    return Ok(*only);
                }
                let dst = self.new_value(
                    NativeType::Obj,
                    Representation::Boxed(Some(TypeShape::String)),
                );
                self.emit(NativeOp::Concat { dst, parts: values });
                Ok(dst)
            }
            WordExpr::Expand { .. } => Err(NativeLoweringDecline::ArgumentExpansion),
            WordExpr::Opaque { .. } => Err(NativeLoweringDecline::OpaqueWord),
        }
    }

    /// A `[…]` word: a native expression when the registry resolves it to the
    /// expression hook over one braced literal and the module keeps `expr`
    /// bound to its builtin, else a nested generic invocation.
    fn lower_command_substitution(
        &mut self,
        spelling: &str,
        source: &SourceSite,
        depth: u32,
    ) -> Result<NativeValueId, NativeLoweringDecline> {
        let inner = nested_words(spelling, source, self.lexer_config)?;
        if self.substitution_is_native_expression(&inner)
            && inner.len() == 2
            && let WordExpr::BracedLiteral { text, .. } = &inner[1]
        {
            let expr = tcl_syntax::expr::parser::parse_expr_for_profile(text, self.dialect);
            let value = self.lower_expression_text(&expr, text);
            return Ok(self.boxed(value));
        }
        let mut argv = Vec::with_capacity(inner.len());
        for word in &inner {
            argv.push(self.lower_word(word, depth + 1)?);
        }
        self.max_argc = self.max_argc.max(argv.len());
        let dst = self.new_value(NativeType::Obj, Representation::Boxed(None));
        self.emit(NativeOp::NestedInvoke { dst, argv });
        self.clobber_shadows();
        self.note_observation();
        Ok(dst)
    }

    fn substitution_is_native_expression(&self, inner: &[WordExpr]) -> bool {
        if self.source_absent_model() {
            let Ok(RegistryInvocationResolution::Resolved(facts)) =
                resolve_word_exprs(self.input.registry, self.input.context, inner)
            else {
                return false;
            };
            return self.input.registry.native_lowering_for_descriptor(
                facts.descriptor_key(),
                self.input.registry.own_surface_query().as_ref(),
            ) == Some(NativeLowering::Structured(LoweringHookId::Expr))
                && self.command_trusted(facts.descriptor_key());
        }
        let Some(tokens) = self
            .original_invocations
            .as_ref()
            .and_then(|inventory| inventory.substitution(inner))
        else {
            return false;
        };
        let Some(metadata) = self.original_invocation_metadata(tokens) else {
            return false;
        };
        let Some(selected) = resolved_tokens_invocation_with_metadata_context(
            self.input.registry,
            Some(metadata),
            tokens,
        ) else {
            return false;
        };
        self.input.registry.native_lowering_for_descriptor(
            selected.facts.descriptor_key(),
            Some(&metadata.context().authoring_query()),
        ) == Some(NativeLowering::Structured(LoweringHookId::Expr))
            && self.command_trusted(selected.facts.descriptor_key())
    }

    /// Whether the module keeps `name` bound to its registry builtin at every
    /// point, so a nested command word may take the builtin's native shape.
    fn command_trusted(&self, descriptor: tcl_registry::CanonicalCommandDescriptorKey<'_>) -> bool {
        self.mutations.trusts_unobserved_descriptor(
            descriptor,
            self.input.module.has_dynamic_trace,
            &self.input.module.traced_commands,
        )
    }
}

pub(super) fn variable_place(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CellPlace, NativeLoweringDecline> {
    variable_word_place(spelling, source, config).map_err(|decline| match decline {
        VariableWordDecline::Dynamic => NativeLoweringDecline::DynamicVariableName,
        VariableWordDecline::Ambiguous => NativeLoweringDecline::AmbiguousVariableSpelling,
    })
}

/// The structured words of a `[…]` command substitution, recovered by the
/// canonical segmenter over the recorded lexical extent
/// ([`crate::word_subst::nested_command_words`] owns that recovery, so this
/// tier plans the same words the analysis tier lifts).
fn nested_words(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<Vec<WordExpr>, NativeLoweringDecline> {
    crate::word_subst::nested_command_words(spelling, source, config)
        .map(|tokens| tokens.word_exprs)
        .map_err(|decline| match decline {
            crate::word_subst::NestedWordsDecline::Unmodelled => {
                NativeLoweringDecline::UnmodelledCommandSubstitution
            }
            crate::word_subst::NestedWordsDecline::NoWords => {
                NativeLoweringDecline::MissingCommandTokens
            }
        })
}
