// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original method source bodies without an allocated class or receiver frame.

use super::{ClassDef, MemberSide, OriginalSourceMethodMetadata};
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Token};

/// A selected original method declaration owns its borrowed literal body.
/// This receipt issues no runtime class, method, activation, namespace or cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceReceiverBodyDeclaration {
    method: Box<OriginalSourceMethodMetadata>,
    parameters: NativeWord,
    body: NativeWord,
    input: ResolvedAnalysisInput,
}
impl OriginalSourceReceiverBodyDeclaration {
    pub(crate) fn from_class_body(
        class: &ClassDef,
        body: &Token,
        parameters: Option<&Token>,
        input: ResolvedAnalysisInput,
        image: &SourceImage,
    ) -> Option<Arc<Self>> {
        // naming.tcloo.original-source-receiver-body-declaration
        // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
        let parameters = parameters?;
        let config = input.lexer_config();
        let mut candidates = class.original_members.declarations().filter(|method| {
            method.name_purpose() == tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod
                && method.source_dialect().is_some()
                && method.body_word().is_some_and(|word| {
                    word.image() == image
                        && word.config() == config
                        && !word.group().expand
                        && word.tokens().contains(body)
                })
                && method.parameters_word().is_some_and(|word| {
                    word.image() == image
                        && word.config() == config
                        && !word.group().expand
                        && word.tokens().contains(parameters)
                })
        });
        let method = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        let parameters = method.parameters_word()?.clone();
        let body = method.body_word()?.clone();
        let span = body.content_span().ok()?;
        let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(&body)?;
        if image.bytes().get(span.as_range())? != value.as_slice() {
            return None;
        }
        Some(Arc::new(Self {
            method: Box::new(method.clone()),
            parameters,
            body,
            input,
        }))
    }

    /// Canonical selected method metadata, independent of a runtime allocation.
    #[must_use]
    pub fn method(&self) -> &OriginalSourceMethodMetadata {
        &self.method
    }
    /// Original declaration worker site; no dispatch point is fabricated.
    #[must_use]
    pub fn declaration_site(&self) -> &crate::command_binding::CommandAllocationSite {
        self.method.declaration().site()
    }
    /// Independently retained member side, without an entered receiver.
    #[must_use]
    pub fn side(&self) -> MemberSide {
        self.method.side()
    }
    /// Actual complete parameter-list source role, without installed formals.
    #[must_use]
    pub const fn parameters_word(&self) -> &NativeWord {
        &self.parameters
    }
    /// Actual borrowed literal script argument and its original delimiters.
    #[must_use]
    pub const fn body_word(&self) -> &NativeWord {
        &self.body
    }
    /// Exact immutable source, Registry, availability context and full grammar.
    #[must_use]
    pub const fn resolved_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Same source channel/image and complete current analysis input.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.input == *input
            && self.body.image() == image
            && self.parameters.image() == image
            && self.body.config() == input.lexer_config()
            && self.parameters.config() == input.lexer_config()
            && self.declaration_site().source.source_image() == image
    }
    /// A complete original invocation belongs to this exact borrowed body.
    /// Containment supplies source applicability only, never body execution.
    #[must_use]
    pub fn owns_invocation(&self, original: &[NativeWord], input: &ResolvedAnalysisInput) -> bool {
        let Ok(span) = self.body.content_span() else {
            return false;
        };
        !original.is_empty()
            && self.matches_source(self.body.image(), input)
            && original.iter().all(|word| {
                word.image() == self.body.image()
                    && word.config() == input.lexer_config()
                    && word.span().start() >= span.start()
                    && word.span().end() <= span.end()
            })
            && tcl_lexer::native_script_words_in(
                self.body.image().clone(),
                span,
                input.lexer_config(),
            )
            .is_ok_and(|plan| {
                plan.fatal_tail.is_none()
                    && plan
                        .commands
                        .iter()
                        .any(|command| command.words == original)
            })
    }
}

#[cfg(test)]
mod tests {
    use tcl_lexer::SourceImage;
    fn methods(scope: &super::super::Scope) -> Vec<&super::OriginalSourceReceiverBodyDeclaration> {
        scope
            .original_receiver_body_declaration
            .iter()
            .map(AsRef::as_ref)
            .chain(scope.children.iter().flat_map(methods))
            .collect()
    }

    #[test]
    fn original_source_receiver_body_retains_child_declaration_without_native_entry() {
        // naming.tcloo.original-source-receiver-body-declaration
        // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
        let source = "interp create -safe s; interp eval s {oo::class create C {method m {} {source a.tcl}}}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let bodies = methods(&analysis.global_scope);
        assert_eq!(bodies.len(), 1);
        let body = bodies[0];
        let image = SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            body.body_word().content_span().unwrap(),
            body.resolved_input().lexer_config(),
        )
        .unwrap();
        assert!(body.matches_source(&image, body.resolved_input()));
        assert!(body.owns_invocation(&plan.commands[0].words, body.resolved_input()));
        assert!(!body.owns_invocation(&plan.commands[0].words[..1], body.resolved_input()));
        assert_eq!(body.method().original_name_input().bytes(), b"m");
        assert_eq!(body.side(), crate::analyser::MemberSide::Instance);
        assert!(!body.matches_source(
            &SourceImage::document(&(source.to_owned() + " ")),
            body.resolved_input()
        ));
        let root = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            body.resolved_input().lexer_config(),
        )
        .unwrap();
        assert!(!body.owns_invocation(&root.commands[0].words, body.resolved_input()));
    }

    #[test]
    fn original_source_receiver_body_refuses_computed_or_recooked_script_mapping() {
        // naming.tcloo.original-source-receiver-body-declaration
        // docs/design/analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md
        for source in [
            "oo::class create C {method m {} $script}",
            "oo::class create C {method m {} \"source\\ta.tcl\"}",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(methods(&analysis.global_scope).is_empty(), "{source}");
        }
    }
}
