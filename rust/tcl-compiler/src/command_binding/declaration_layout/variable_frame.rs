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

//! The original variable frame of an authored or Logical source and its direct script cursor.

use super::SourceOriginalVariableFrame;
use std::sync::Arc;

impl std::hash::Hash for SourceOriginalVariableFrame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.source, state);
        std::hash::Hash::hash(&self.frame, state);
        std::hash::Hash::hash(&self.config, state);
    }
}

impl SourceOriginalVariableFrame {
    // Conditional authored procedure layout only; no entered frame or cell.
    pub(crate) fn from_authored_scalar_source(
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let binding = tokens.source_binding.as_ref()?;
        let scope = binding.original_authored_scalar_body_scope(tokens, registry)?;
        scope.original_words(tokens, registry)?;
        scope.scalar_name_policy()?;
        let advice = binding.original_authored_declaration_scalar_layout(tokens, registry)?;
        if advice.has_unclosed_source_coverage()
            || scope.has_unclosed_declaration_source_coverage()
            || advice.namespace() != scope.namespace()
        {
            return None;
        }
        let body = scope.scalar_frame_body();
        let span = body.content_span().ok()?;
        let origin = &binding.invocation_site()?.source;
        (origin.source_image() == body.image()).then_some(())?;
        let text = body.image().try_text().ok()?.get(span.as_range())?;
        Some(Self {
            source: super::super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                tcl_lexer::SourceImage::document(text),
                span.start(),
            )?,
            frame: advice.frame().clone(),
            config: scope.config(),
            logical_lambda: None,
        })
    }

    /// A release unit's procedure body frame under its authored simulation:
    /// the procedure's executed body under its own namespace, with the
    /// activation identity the walk gives the declaration's frame. The formal
    /// list must parse strictly to the declaration's own parameters, each a
    /// unique plain local. Conditional layout only; no entered frame or cell.
    pub(crate) fn from_authored_procedure(
        procedure: &crate::ir::Procedure,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
        let policy = super::super::source_analysis_entry::source_input_dialect(input)
            .authored_name_policy()?;
        if policy.authority() != NamePolicyAuthority::AuthoredSimulation
            || !matches!(policy.recipe(), NativeNameProtocol::C(_))
        {
            return None;
        }
        let parameters = super::super::logical_definition::static_formals(
            &procedure.params_raw,
            input.lexer_config().list_parse,
        )?;
        if !parameters
            .iter()
            .map(|formal| formal.name.as_str())
            .eq(procedure.params.iter().map(String::as_str))
            || !super::super::logical_definition::local_formals_are_unique(&parameters, policy)
        {
            return None;
        }
        let source = procedure.body.executed_source.as_deref()?.clone();
        let namespace = procedure.body.namespace_context.as_deref()?.clone();
        let site = procedure.declaration_site.as_ref()?;
        Some(Self {
            source,
            frame: crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: namespace.display()?,
                identity: super::super::source_activation_name(
                    Some(&site.source),
                    &procedure.qualified_name,
                    site.offset,
                ),
            }
            .with_namespace_identity(namespace),
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    /// Conditional lexical lambda layout from its held call and source fields.
    /// This does not issue an entered frame, installed local or Native recipe.
    pub(crate) fn from_logical_lambda_source(
        receipt: Arc<super::super::OriginalLogicalLambdaBody>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let (origin, offset, body, namespace) = receipt.frame_source(input)?;
        let crate::signature_scan::scope::SignatureNamespaceScope::C(path) = namespace else {
            return None;
        };
        let namespace = super::super::SourceNamespaceKey::authored(
            tcl_syntax::naming::checked_namespace_path_key(&path)?,
        );
        let text = origin
            .source_image()
            .try_text()
            .ok()?
            .get(body.as_range())?;
        let source = super::super::ExecutedScriptSource::contiguous_image(
            Arc::clone(&origin),
            tcl_lexer::SourceImage::document(text),
            body.start(),
        )?;
        Some(Self {
            source,
            frame: crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: namespace.display()?,
                identity: super::super::source_activation_name(
                    Some(&origin),
                    "logical-lambda-layout",
                    offset,
                ),
            }
            .with_namespace_identity(namespace),
            config: input.lexer_config(),
            logical_lambda: Some(receipt),
        })
    }

    pub(crate) fn from_logical_procedure_source(
        card: &super::super::OriginalLogicalProcedureBody,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        let image = card.site().source.source_image();
        if !super::super::source_analysis_entry::source_input_dialect(input)
            .authored_name_policy()
            .is_some_and(|policy| {
                policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                    && matches!(
                        policy.recipe(),
                        tcl_syntax::naming::NativeNameProtocol::C(_)
                    )
            })
            || !card.matches_source(image, input)
            || card.scope().frame_recipe()
                != super::super::OriginalDeclaredSourceBodyFrame::Procedure
        {
            return None;
        }
        let body = card.scope().scope_body().content_span();
        let text = image.try_text().ok()?.get(body.as_range())?;
        let source = super::super::ExecutedScriptSource::contiguous_image(
            Arc::clone(&card.site().source),
            tcl_lexer::SourceImage::document(text),
            body.start(),
        )?;
        let namespace = super::super::SourceNamespaceKey::authored(
            tcl_syntax::naming::checked_namespace_path_key(card.scope().namespace_recipe())?,
        );
        Some(Self {
            source,
            frame: super::super::declaration_preview::declared_procedure_frame(
                card.site(),
                &namespace,
            ),
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    pub(in super::super) fn from_logical_namespace_source(
        scope: &super::super::OriginalDeclaredLogicalBodyContext,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Self> {
        if scope.frame_recipe() != super::super::OriginalDeclaredSourceBodyFrame::Namespace
            || !scope.matches_input(input)
            || !super::super::source_analysis_entry::source_input_dialect(input)
                .authored_name_policy()
                .is_some_and(|policy| {
                    policy.authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                        && matches!(
                            policy.recipe(),
                            tcl_syntax::naming::NativeNameProtocol::C(_)
                        )
                })
        {
            return None;
        }
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = scope.scope_body().source_words().source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input) {
            return None;
        }
        let origin = &advice.site().source;
        let image = origin.source_image();
        let body = scope.scope_body().content_span();
        let text = image.try_text().ok()?.get(body.as_range())?;
        let namespace = tcl_syntax::naming::checked_namespace_path_key(scope.namespace_recipe())?;
        Some(Self {
            source: super::super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                tcl_lexer::SourceImage::document(text),
                body.start(),
            )?,
            frame: crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity: format!(
                    "logical-namespace-source:{}:{}",
                    super::super::SourceOriginId::activation_key(origin),
                    body.start()
                ),
            },
            config: input.lexer_config(),
            logical_lambda: None,
        })
    }

    /// Direct cursor membership in an independently selected same-frame source
    /// region. The caller retains its scope/frame recipe; this checks only the
    /// actual region owner and its containment in this held frame source.
    pub(crate) fn source_region_cursor_is_direct(
        &self,
        image: &tcl_lexer::SourceImage,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
        offset: u32,
    ) -> bool {
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = region.source_words().source()
        else {
            return false;
        };
        let span = region.content_span();
        let Some(end) = u32::try_from(self.source.text.bytes().len())
            .ok()
            .and_then(|len| self.source.base().checked_add(len))
        else {
            return false;
        };
        Arc::ptr_eq(&advice.site().source, &self.source.origin)
            && region.matches_source(image, self.config)
            && self.source.origin.source_image() == image
            && self.source.base() <= span.start()
            && span.end() <= end
            && span.start() <= offset
            && offset <= span.end()
            && direct_script_cursor(image, self.config, span, offset, 0)
    }

    /// Conditional procedure, method or lambda layout selected by its source
    /// body recipe. This supplies no entered activation or physical local table.
    #[must_use]
    pub const fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    /// Complete retained source owner and its truthful body mapping.
    #[must_use]
    pub const fn source(&self) -> &super::super::ExecutedScriptSource {
        &self.source
    }

    /// Original full parser configuration retained with the frame recipe.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// A braced operand without its own selected body receipt cannot inherit the
/// enclosing procedure's locals. Command substitutions retain the same frame.
pub(super) fn direct_script_cursor(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    span: tcl_lexer::Span,
    offset: u32,
    depth: usize,
) -> bool {
    if depth > 64 {
        return false;
    }
    let Ok(plan) = tcl_lexer::native_script_words_in(image.clone(), span, config) else {
        return false;
    };
    if plan.fatal_tail.is_some() {
        return false;
    }
    for command in plan.commands {
        for word in command.words {
            if word.group().kind == tcl_lexer::WordKind::Braced
                && word
                    .content_span()
                    .is_ok_and(|content| content.start() <= offset && offset <= content.end())
            {
                return false;
            }
            let arena = word.executable_parts();
            for part in arena.all_parts() {
                if let tcl_lexer::ExecutablePart::Command { body } = part.part
                    && body.start() <= offset
                    && offset <= body.end()
                {
                    return direct_script_cursor(image, config, body, offset, depth + 1);
                }
            }
        }
    }
    true
}
