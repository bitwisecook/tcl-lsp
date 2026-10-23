// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hosted declaration bodies retain source scope without publication or entry.

use super::{
    CommandAllocationSite, ExecutedScriptSource, SourceCommandBindings, SourceNamespaceKey,
    SourceRuleDeclarationCandidate,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::signature_scan::vendor_name::VendorSourceNameInput;
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span};
use tcl_syntax::naming::{VendorSourceFormalExtent, VendorSourceNamePurpose};

/// An actual rule-loader declaration's conditional body and authored scope.
/// Unknown handlers remain retained; this cannot issue a Native namespace,
/// parameter binder, installed procedure, body entry or rewrite permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalHostedProcedureBody {
    site: CommandAllocationSite,
    input: ResolvedAnalysisInput,
    words: Arc<[NativeWord]>,
    declaration: tcl_registry::events::IrulesTopLevelDeclaration,
    alternatives: Vec<SourceRuleDeclarationCandidate>,
    source: ExecutedScriptSource,
    namespace: SourceNamespaceKey,
    name: VendorSourceNameInput,
    parameters: Vec<VendorSourceFormalExtent>,
    parameter_word: NativeWord,
    span: Span,
    lowered_body: Option<Arc<crate::ir::Script>>,
    retained_entry: Option<super::SourceAnalysisEntry>,
}
impl OriginalHostedProcedureBody {
    pub(crate) fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    pub(crate) fn source(&self) -> &ExecutedScriptSource {
        &self.source
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn name(&self) -> Option<&str> {
        std::str::from_utf8(
            self.name
                .literal_units(VendorSourceNamePurpose::ProcedureName)?,
        )
        .ok()
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        std::str::from_utf8(tcl_syntax::naming::vendor_source_literal_units(
            self.name.policy(),
            &self.parameter_word,
            VendorSourceNamePurpose::FormalList,
        )?)
        .ok()
    }
    pub(crate) fn parameter_names(&self) -> Option<Vec<String>> {
        let units = self.parameters_text()?;
        self.parameters
            .iter()
            .map(|formal| Some(units.get(formal.name.clone())?.to_owned()))
            .collect()
    }
    pub(crate) fn span(&self) -> Span {
        self.span
    }
    pub(crate) fn retain_lowered_body(
        mut self,
        body: &crate::ir::Script,
        entry: &super::SourceAnalysisEntry,
    ) -> Self {
        self.lowered_body = Some(Arc::new(body.clone()));
        self.retained_entry = Some(entry.clone());
        self
    }
    pub(crate) fn matches_declaration(
        &self,
        declaration: &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    ) -> bool {
        let occurrence = declaration.original_occurrence();
        declaration.purpose() == VendorSourceNamePurpose::ProcedureName
            && declaration.name_input() == &self.name
            && occurrence.site() == &self.site
            && occurrence.original_words() == self.words.as_ref()
            && declaration.metadata().body_span.start() == self.source.base()
            && self
                .site
                .source
                .source_image()
                .try_text()
                .ok()
                .and_then(|source| source.get(declaration.metadata().body_span.as_range()))
                == self.source.try_text().ok()
    }

    pub(crate) fn matches_procedure(
        &self,
        module: &crate::ir::Module,
        procedure: &crate::ir::Procedure,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return false;
        };
        self.retained_entry.as_ref() == Some(&module.source_entry)
            && module.source_entry.native_entry.is_none()
            && module.lexer_config == input.lexer_config()
            && module.registry_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.semantic_key() == registry.snapshot().semantic_key()
            })
            && self.owns_procedure(procedure, &module.source, input, registry)
    }
    pub(crate) fn owns_procedure(
        &self,
        procedure: &crate::ir::Procedure,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.matches_source(image, input, registry)
            && procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
            && procedure.publication.is_none()
            && procedure.declaration_site.as_ref() == Some(&self.site)
            && procedure.span == self.span
            && procedure.body_offset == self.source.base()
            && procedure.body.namespace_context.as_deref() == Some(&self.namespace)
            && self.parameter_names().as_ref() == Some(&procedure.params)
            && self.parameters_text() == Some(procedure.params_raw.as_str())
            && self.source.try_text().ok() == procedure.body_source.as_deref()
            && self.lowered_body.as_deref() == Some(&procedure.body)
    }
    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        &self.input == input
            && input.has_hosted_source_name_context()
            && input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                == registry.snapshot().semantic_key()
            && self.site.source.source_image() == image
            && self
                .words
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
    }
}
fn hosted_declaration_row_matches(
    row: &super::declaration_layout::DeclarationLayoutObservation,
    input: &ResolvedAnalysisInput,
) -> bool {
    !(row.config != input.lexer_config()
        || row.snapshot.realm != tcl_dialect::model::InvocationRealm::RuleLoader
        || row.snapshot.state.baseline.vendor_source_input.as_ref() != Some(input)
        || row.snapshot.state.baseline.native_entry.is_some()
        || !matches!(
            row.entry.as_ref(),
            super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                frame: crate::var_resolve::VariableExecutionFrame::Global,
                ..
            }
        )
        || !row.namespace.is_root())
}

fn matching_hosted_body_source(
    alternatives: &[SourceRuleDeclarationCandidate],
    declaration: &tcl_registry::events::IrulesTopLevelDeclaration,
    command: &str,
    site: &CommandAllocationSite,
    image: &SourceImage,
    content: Span,
    registry: &tcl_registry::CommandRegistry,
) -> Option<ExecutedScriptSource> {
    let mut source = None;
    for alternative in alternatives {
        if &alternative.declaration != declaration
            || !alternative.target.registry_backed
            || !alternative.target.prepended.is_empty()
            || registry
                .get(&alternative.target.command)
                .map(|spec| spec.descriptor_key().label())
                != Some(command)
        {
            continue;
        }
        let body = alternative.body.as_ref()?;
        if body.origin != site.source
            || body.base() != content.start()
            || body.text.bytes() != image.bytes().get(content.as_range())?
        {
            return None;
        }
        if source.as_ref().is_some_and(|previous| previous != body) {
            return None;
        }
        source = Some(body.clone());
    }
    source
}

impl SourceCommandBindings {
    /// Exact hosted declaration alternatives retain their original scope and
    /// words. A source declaration does not establish a successful installation.
    pub(crate) fn original_hosted_procedure_body_cards(
        &self,
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<Arc<OriginalHostedProcedureBody>> {
        if !input.has_hosted_source_name_context()
            || !input.unit_profile().is_irules()
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
            || self.final_state.baseline.vendor_source_input.as_ref() != Some(input)
            || self.final_state.baseline.native_entry.is_some()
        {
            return Vec::new();
        }
        self.rule_declarations
            .iter()
            .filter_map(|(site, alternatives)| {
                self.original_hosted_procedure_body_card(site, alternatives, input, registry)
                    .map(Arc::new)
            })
            .collect()
    }

    fn original_hosted_procedure_body_card(
        &self,
        site: &CommandAllocationSite,
        alternatives: &[SourceRuleDeclarationCandidate],
        input: &ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalHostedProcedureBody> {
        if self.root_origin.as_ref()? != &site.source {
            return None;
        }
        let image = site.source.source_image();
        let text = image.try_text().ok()?;
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layouts.get(site)?,
        )?;
        let mut agreed = None;
        for row in rows {
            if !hosted_declaration_row_matches(row, input) {
                return None;
            }
            let mut tokens = super::declaration_preview::declaration_tokens(site, row)?;
            self.stamp_original_tokens(&mut tokens);
            let original = crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                site.offset,
                row.config,
            )?;
            let hosted = tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(&input.borrowed_context_registry().context().environment.id)?;
            let policy = input
                .vendor_source_policy()
                .or_else(|| tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted))?;
            let head = VendorSourceNameInput::from_original_word(original.first()?, policy);
            let metadata =
                crate::registry_invocation::original_conditional_vendor_registry_metadata(
                    input.borrowed_context_registry(),
                    &tokens,
                    &head,
                    &original,
                )?;
            let (end, declaration) = hosted_declaration_shape(
                registry,
                text,
                site,
                row.config,
                &original,
                metadata.shape().command(),
            )?;
            let tcl_registry::events::IrulesTopLevelDeclaration::Procedure {
                name_index,
                body_index,
            } = &declaration
            else {
                return None;
            };
            let name = VendorSourceNameInput::from_original_word(
                original.get(name_index.checked_add(1)?)?,
                policy,
            );
            name.root_procedure_coordinate()?;
            if !metadata.shape().roles_complete() || metadata.shape().argument_offset() != 0 {
                return None;
            }
            let mut parameter_roles = metadata
                .shape()
                .roles()
                .iter()
                .filter(|(_, role)| *role == tcl_registry::ArgRole::ParamList)
                .map(|(index, _)| usize::from(*index));
            let parameter_index = parameter_roles.next()?;
            if parameter_roles.next().is_some() {
                return None;
            }
            let parameter_word = original.get(parameter_index.checked_add(1)?)?.clone();
            let parameters =
                tcl_syntax::naming::vendor_source_formal_extents(policy, &parameter_word)?;
            let body_word = original.get(body_index.checked_add(1)?)?;
            let content = body_word.content_span().ok()?;
            let source = matching_hosted_body_source(
                alternatives,
                &declaration,
                metadata.shape().command(),
                site,
                image,
                content,
                registry,
            )?;
            let card = OriginalHostedProcedureBody {
                site: site.clone(),
                input: input.clone(),
                words: Arc::from(original),
                declaration,
                alternatives: alternatives.to_vec(),
                source,
                namespace: row.namespace.clone(),
                name,
                parameters,
                parameter_word,
                span: Span::new(site.offset, end),
                lowered_body: None,
                retained_entry: None,
            };
            if agreed.as_ref().is_some_and(|previous| previous != &card) {
                return None;
            }
            agreed = Some(card);
        }
        agreed
    }
}

fn hosted_declaration_shape(
    registry: &tcl_registry::CommandRegistry,
    text: &str,
    site: &CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
    original: &[tcl_lexer::NativeWord],
    command: &str,
) -> Option<(u32, tcl_registry::events::IrulesTopLevelDeclaration)> {
    let end = original.last()?.span().end();
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        text.get(site.offset as usize..end as usize)?,
        site.offset,
        config.nested(),
    );
    let [segment] = segments.as_slice() else {
        return None;
    };
    let values = segment
        .args()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let closed = tcl_registry::events::closed_braced_argument_words(
        text,
        segment.arg_tokens(),
        segment.arg_single_token(),
    )?;
    let arguments = tcl_registry::events::IrulesDeclarationArguments::new(
        &values,
        segment.arg_tokens(),
        segment.arg_single_token(),
        &closed,
    )?;
    let declaration = registry.irules_top_level_declaration_shape(command, arguments)?;
    Some((end, declaration))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(source: &str) -> (SourceCommandBindings, ResolvedAnalysisInput) {
        fixture_with_history(source, false)
    }

    /// `unknown_history` supplies the explicit unknown-history entry; without
    /// it a supplied document is a fresh rule load (user decision
    /// 2026-10-21T00:10Z), whose loader `proc` is resolved.
    fn fixture_with_history(
        source: &str,
        unknown_history: bool,
    ) -> (SourceCommandBindings, ResolvedAnalysisInput) {
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            config,
        );
        let registry = input.borrowed_context_registry().commands();
        let entry = if unknown_history {
            super::super::SourceAnalysisEntry::for_hosted_source(&input).expect("hosted F5 input")
        } else {
            super::super::SourceAnalysisEntry::for_supplied_source(
                registry,
                &input,
                config,
                Some(profile),
            )
        };
        let bindings =
            SourceCommandBindings::analyse_with_options(source, config, registry, entry.options());
        (bindings, input)
    }
    #[test]
    fn hosted_body_requires_the_actual_captured_registry_descriptor() {
        let (mut bindings, input) = fixture("proc ordinary {} {return value}");
        let registry = input.borrowed_context_registry().commands();
        let candidates = bindings
            .deferred_rule_declaration_candidates(0)
            .expect("actual retained declaration alternatives");
        assert!(candidates.iter().any(|candidate| {
            registry
                .get(&candidate.target.command)
                .is_some_and(|spec| spec.descriptor_key().label() == "proc")
        }));
        let cards = bindings.original_hosted_procedure_body_cards(&input, registry);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].alternatives, candidates);
        for candidates in bindings.rule_declarations.values_mut() {
            for candidate in candidates {
                candidate.target.command = "::puts".to_owned();
            }
        }
        assert!(
            bindings
                .original_hosted_procedure_body_cards(&input, registry)
                .is_empty()
        );
    }

    #[test]
    fn hosted_declaration_body_keeps_real_words_scope_and_unknown_handlers() {
        let source = "proc ordinary {value {fallback DEFAULT}} {set static::counter 1; return $value}\nwhen HTTP_REQUEST {puts request}\n";
        // Unknown declaration alternatives need unknown prior history; a
        // fresh rule load resolves the loader's `proc`.
        let (bindings, input) = fixture_with_history(source, true);
        let registry = input.borrowed_context_registry().commands();
        let cards = bindings.original_hosted_procedure_body_cards(&input, registry);
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.name(), Some("ordinary"));
        assert_eq!(
            card.parameter_names(),
            Some(vec!["value".into(), "fallback".into()])
        );
        assert_eq!(
            card.source().try_text().unwrap(),
            "set static::counter 1; return $value"
        );
        assert_eq!(card.words.len(), 4);
        assert!(
            card.alternatives
                .as_slice()
                .iter()
                .any(|alternative| alternative.unknown)
        );
        assert!(card.matches_source(&SourceImage::document(source), &input, registry));
        assert!(card.namespace().native_context().is_none());
        assert!(
            bindings
                .invocation_at_source("proc", 0)
                .proved_execution_target()
                .is_none()
        );
        assert!(!card.matches_source(
            &SourceImage::document(&format!("{source} ")),
            &input,
            registry
        ));
        assert!(!card.matches_source(
            &SourceImage::document(source),
            &input,
            tcl_registry::default_registry()
        ));
    }
    #[test]
    fn hosted_declaration_body_refuses_unowned_names_formals_and_other_domains() {
        for source in [
            "proc ::when::RULE_INIT {} {set static::counter 1}",
            "proc $name {} {return VALUE}",
            "proc ordinary {value\\u0061} {return VALUE}",
            "proc ordinary $formals {return VALUE}",
            "proc ordinary {} $body",
        ] {
            let (bindings, input) = fixture(source);
            assert!(
                bindings
                    .original_hosted_procedure_body_cards(
                        &input,
                        input.borrowed_context_registry().commands()
                    )
                    .is_empty(),
                "{source}"
            );
        }
        let source = "proc ordinary {} {return VALUE}";
        let (bindings, input) = fixture(source);
        let environment = tcl_registry::model::ingress::resolve_environment("tcl");
        let profile = environment.unit_profile();
        let logical = ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(
            bindings
                .original_hosted_procedure_body_cards(
                    &logical,
                    input.borrowed_context_registry().commands()
                )
                .is_empty()
        );
        let other = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            environment.default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            bindings
                .original_hosted_procedure_body_cards(
                    &other,
                    input.borrowed_context_registry().commands()
                )
                .is_empty()
        );
    }
}
