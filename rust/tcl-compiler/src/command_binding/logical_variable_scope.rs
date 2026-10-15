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

//! Original Logical source naming scope, independent of Native variable selection.

use super::{
    OriginalDeclaredLogicalBodyContext, OriginalDeclaredSourceBodyFrame,
    OriginalLogicalProcedureBody, SourceOriginId, SourceOriginalVariableFrame,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
enum LogicalVariableFrameRecipe {
    Document(Arc<SourceOriginId>),
    Declared {
        scope: Arc<OriginalDeclaredLogicalBodyContext>,
        procedure: Option<Arc<OriginalLogicalProcedureBody>>,
    },
}

/// Actual source scope and complete original interpretation. This is a naming
/// recipe, never a selected variable cell, entered frame or rename certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalVariableScope {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    frame: LogicalVariableFrameRecipe,
    earlier_unknown_mutations: bool,
}

impl OriginalLogicalVariableScope {
    pub(super) fn document(
        input: &ResolvedAnalysisInput,
        origin: Arc<SourceOriginId>,
        original: Arc<[NativeWord]>,
        earlier_unknown_mutations: bool,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> Option<Self> {
        let result = Self {
            input: input.clone(),
            original,
            frame: LogicalVariableFrameRecipe::Document(origin),
            earlier_unknown_mutations,
        };
        result.matches_name(span, name).then_some(result)
    }

    pub(super) fn declared(
        input: &ResolvedAnalysisInput,
        scope: Arc<OriginalDeclaredLogicalBodyContext>,
        procedure: Option<Arc<OriginalLogicalProcedureBody>>,
        original: Arc<[NativeWord]>,
        earlier_unknown_mutations: bool,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> Option<Self> {
        if scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Procedure {
            let card = procedure.as_ref()?;
            if !same_procedure_scope(card, &scope)
                || !card.matches_source(original.first()?.image(), input)
            {
                return None;
            }
        } else if procedure.is_some() {
            return None;
        }
        let result = Self {
            input: input.clone(),
            original,
            frame: LogicalVariableFrameRecipe::Declared { scope, procedure },
            earlier_unknown_mutations,
        };
        result.matches_name(span, name).then_some(result)
    }

    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.input == *input
            && input.has_logical_source_name_context()
            && super::source_analysis_entry::source_input_dialect(input)
                .authored_name_policy()
                .is_some_and(|policy| {
                    policy.authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                        && matches!(
                            policy.recipe(),
                            tcl_syntax::naming::NativeNameProtocol::C(_)
                        )
                })
            && !self.original.is_empty()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && match &self.frame {
                LogicalVariableFrameRecipe::Document(origin) => origin.source_image() == image,
                LogicalVariableFrameRecipe::Declared { scope, procedure } => {
                    scope.matches_input(input)
                        && scope.owns_words(&self.original)
                        && procedure.as_ref().is_none_or(|card| {
                            same_procedure_scope(card, scope) && card.matches_source(image, input)
                        })
                }
            }
    }

    pub(crate) fn matches_name(&self, span: Span, name: &SignatureSourceNameInput) -> bool {
        let Some(image) = self.original.first().map(NativeWord::image) else {
            return false;
        };
        if !self.matches_source(image, &self.input)
            || super::source_analysis_entry::source_input_dialect(&self.input)
                .authored_name_policy()
                != Some(name.policy())
            || name.policy().authority()
                != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        {
            return false;
        }
        Self::original_words_match_name(&self.original, &self.input, span, name)
    }

    pub(super) fn original_words_match_name(
        original: &[NativeWord],
        input: &ResolvedAnalysisInput,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> bool {
        let Some(image) = original.first().map(NativeWord::image) else {
            return false;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config());
        match name {
            SignatureSourceNameInput::OriginalWord(key) => original.iter().any(|word| {
                (word.span() == span
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span == span))
                    && SignatureSourceNameKey::from_original_logical_source_word(
                        word,
                        rules,
                        name.policy(),
                    )
                    .as_ref()
                        == Some(key)
            }),
            SignatureSourceNameInput::OriginalVariableRoot(root) => {
                root.source_span() == span
                    && root.source_image() == image
                    && root.lexer_config() == input.lexer_config()
                    && original.iter().any(|word| {
                        let arena = word.executable_parts();
                        let mut parts = arena.all_parts().filter(|part| {
                            matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
                                && part.span == root.part_span()
                                && arena.source_span(part) == Some(span)
                        });
                        let Some(part) = parts.next() else {
                            return false;
                        };
                        let tcl_lexer::ExecutablePart::Variable { name, .. } = part.part else {
                            return false;
                        };
                        parts.next().is_none() && arena.bytes(name) == Some(root.bytes())
                    })
            }
            SignatureSourceNameInput::OriginalValue(_) => false,
        }
    }

    pub(crate) fn namespace(&self) -> SignatureNamespaceScope {
        SignatureNamespaceScope::C(match &self.frame {
            LogicalVariableFrameRecipe::Document(_) => tcl_core_types::ByteNamespacePath::root(),
            LogicalVariableFrameRecipe::Declared { scope, .. } => scope.namespace_recipe().clone(),
        })
    }

    pub(crate) fn local_frame(&self) -> Option<SourceOriginalVariableFrame> {
        let LogicalVariableFrameRecipe::Declared {
            procedure: Some(card),
            ..
        } = &self.frame
        else {
            return None;
        };
        SourceOriginalVariableFrame::from_logical_procedure_source(card, &self.input)
    }

    pub(crate) fn namespace_storage(&self) -> bool {
        match &self.frame {
            LogicalVariableFrameRecipe::Document(_) => true,
            LogicalVariableFrameRecipe::Declared { scope, .. } => {
                scope.frame_recipe() != OriginalDeclaredSourceBodyFrame::Procedure
            }
        }
    }

    pub(crate) fn local_name_is_byte_unique(&self, name: &[u8]) -> bool {
        let LogicalVariableFrameRecipe::Declared {
            procedure: Some(card),
            ..
        } = &self.frame
        else {
            return false;
        };
        let Some(policy) =
            super::source_analysis_entry::source_input_dialect(&self.input).authored_name_policy()
        else {
            return false;
        };
        let Some(parameters) = super::logical_definition::static_formals(
            card.parameters_text(),
            self.input.lexer_config().list_parse,
        ) else {
            return false;
        };
        if parameters != card.parameters() {
            return false;
        }
        let protocol = policy.recipe();
        let mut seen = std::collections::HashSet::new();
        parameters.iter().all(|formal| {
            let input = protocol.combined_variable_input(formal.name.as_bytes());
            input.element().is_none()
                && matches!(protocol.variable_root_geometry(tcl_syntax::naming::NativeNameContext::root(), input.root().selected()),
                    tcl_syntax::naming::NativeVariableRootGeometry::Local(simple) if simple.as_bytes() == formal.name.as_bytes())
                && seen.insert(formal.name.as_bytes())
        }) && !name.contains(&0)
    }

    pub(crate) const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }
}

fn same_procedure_scope(
    card: &OriginalLogicalProcedureBody,
    scope: &OriginalDeclaredLogicalBodyContext,
) -> bool {
    std::ptr::eq(card.scope().scope_body(), scope.scope_body())
        && card.scope().scope_name_input() == scope.scope_name_input()
        && card.scope().namespace_recipe() == scope.namespace_recipe()
        && card.scope().frame_recipe() == scope.frame_recipe()
}
