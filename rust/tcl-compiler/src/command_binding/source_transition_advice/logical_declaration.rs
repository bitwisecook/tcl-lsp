// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Positioned Logical schemas retain the genuine conditional declaration entry.

use super::{
    AdviceNamingPolicy, OriginalSourceCommandTransitionAdvice, SourceCommandTransitionObligation,
};
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::model::ContextRegistry;

impl crate::command_binding::SourceCommandBindings {
    pub(crate) fn original_logical_declaration_advice(
        &self,
        context: &ContextRegistry,
        original: &[NativeWord],
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let input = self.original_logical_source_name_advice_input()?;
        if !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context) {
            return None;
        }
        let (command, declaration) = self.original_logical_source_header_rows(input, original)?;
        let dialect = declaration.first()?.snapshot.state.baseline.dialect?;
        if declaration
            .iter()
            .any(|row| row.snapshot.state.baseline.dialect != Some(dialect))
        {
            return None;
        }
        let policy = AdviceNamingPolicy::Logical(input.clone());
        let mut written = super::original_words(original, &policy)?;
        let head = written.first()?.input.as_ref()?.clone();
        written.remove(0);
        let words = super::registry_words(&written);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        let (roles, complete) = schema.authored_source_argument_roles();
        let roles = complete
            .then(|| {
                roles
                    .into_iter()
                    .map(|(ordinal, role)| {
                        schema
                            .semantics
                            .argument_offset
                            .checked_add(usize::from(ordinal))
                            .map(|ordinal| (ordinal, role))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten();
        let command = schema.canonical_command.to_owned();
        Some(OriginalSourceCommandTransitionAdvice {
            site: crate::command_binding::CommandAllocationSite {
                source: Arc::clone(self.root_origin.as_ref()?),
                offset: original.first()?.span().start(),
            },
            original: Arc::from(original),
            head,
            arguments: written,
            command,
            context: context.context().clone(),
            registry: context.commands().snapshot().semantic_key(),
            config: input.lexer_config(),
            dialect,
            roles,
            lineage: Vec::new(),
            obligations: vec![
                SourceCommandTransitionObligation::UnavailableActualLookup,
                SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability,
                SourceCommandTransitionObligation::ConditionalDeclarationApplicability,
            ],
            uncertain_operations: Vec::new(),
            logical_input: Some(input.clone()),
            declaration: Some(declaration),
            logical_body: None,
            procedure_body: None,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_logical_nested_header_retains_declared_body_and_full_input() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "proc p {longname} {return $longname}\n";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let realm = analysis.retained_command_realm().unwrap();
        let root = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let body = root.commands[0].words[3].content_span().unwrap();
        let nested = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            body,
            config,
        )
        .unwrap()
        .commands
        .remove(0);
        let words = crate::registry_invocation::source_structure::original_logical_registry_words_for_command(
            realm, &input, &nested,
        ).expect("genuine conditional procedure body header");
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            panic!("missing Logical declaration purpose");
        };
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(advice.declaration.is_none());
        let body_owner = advice
            .logical_source_body()
            .expect("original Logical procedure body");
        assert_eq!(body_owner.content_span(), body);
        assert!(body_owner.matches_source(&tcl_lexer::SourceImage::document(source), config));
        assert!(
            advice.obligations().contains(
                &super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability
            )
        );
        assert!(!words.operands_preserve_source_lookup());
        assert_eq!(advice.original_words(), nested.words.as_slice());
        assert!(
            words
                .with_source_schema(&context, |schema| schema
                    .canonical_command
                    .ends_with("return"))
                .unwrap()
        );
        let shadow = "proc return {args} {}; proc p {longname} {return $longname}\n";
        let blocked = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(shadow, profile.name);
        let start = shadow.rfind("return $longname").unwrap();
        let region = tcl_lexer::Span::new(
            u32::try_from(start).unwrap(),
            u32::try_from(start + "return $longname".len()).unwrap(),
        );
        let nested = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(shadow),
            region,
            config,
        )
        .unwrap()
        .commands
        .remove(0);
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_command(
            blocked.retained_command_realm().unwrap(), &input, &nested,
        ).is_none());
    }
}
