// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hosted variable source advice without runtime cell or native frame authority.

use super::vendor_name::VendorSourceNameOccurrence;
use tcl_lexer::{LexerConfig, SourceImage, Span, TokenType};
use tcl_registry::resolved_invocation::VariableReceiverOperandForm;
use tcl_syntax::naming::{VendorSourceFormalExtent, VendorSourceNamePurpose};

/// Independently selected Registry body role, for source visibility only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorSourceVariableBodyKind {
    /// The selected procedure declaration's original braced body.
    Procedure,
    /// The selected namespace script's original braced body.
    Namespace,
    /// The selected event declaration's original braced body.
    Event,
}

/// Actual hosted declaration and body words. This lexical identity supplies
/// neither an entered frame nor a namespace, event or TMM storage lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceVariableBody {
    declaration: VendorSourceNameOccurrence,
    body: VendorSourceNameOccurrence,
    parameters: Option<VendorSourceNameOccurrence>,
    kind: VendorSourceVariableBodyKind,
    span: Span,
    metadata: std::sync::Arc<crate::registry_invocation::VendorRegistryInvocationShape>,
}

impl VendorSourceVariableBody {
    pub(crate) fn new(
        declaration: &VendorSourceNameOccurrence,
        body: &VendorSourceNameOccurrence,
        parameters: Option<&VendorSourceNameOccurrence>,
        kind: VendorSourceVariableBodyKind,
        metadata: crate::registry_invocation::VendorRegistryInvocationShape,
    ) -> Option<Self> {
        let input = body.name_input();
        let words = metadata.original_words();
        let selected_role = |original: &VendorSourceNameOccurrence, role| {
            let Some(written) = words
                .iter()
                .position(|word| word == original.name_input().original_word())
            else {
                return false;
            };
            metadata.roles().iter().any(|(argument, candidate)| {
                *candidate == role
                    && metadata
                        .argument_offset()
                        .checked_add(usize::from(*argument))
                        .and_then(|argument| argument.checked_add(1))
                        == Some(written)
            })
        };
        let selected_header = match kind {
            VendorSourceVariableBodyKind::Procedure => {
                metadata
                    .possible_traits()
                    .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
                    && selected_role(declaration, tcl_registry::ArgRole::Name)
            }
            VendorSourceVariableBodyKind::Namespace => {
                selected_role(declaration, tcl_registry::ArgRole::NamespaceName)
            }
            VendorSourceVariableBodyKind::Event => {
                metadata
                    .possible_traits()
                    .contains(tcl_registry::Traits::IS_EVENT_HANDLER)
                    && words.first() == Some(declaration.name_input().original_word())
            }
        };
        if input.original_word().tokens().first()?.kind != TokenType::Str
            || !metadata.roles_complete()
            || words != body.original_words()
            || metadata.original_head().policy() != input.policy()
            || words.iter().any(|word| word.group().expand)
            || !selected_header
            || !selected_role(body, tcl_registry::ArgRole::Body)
            || (kind == VendorSourceVariableBodyKind::Procedure) != parameters.is_some()
            || parameters.is_some_and(|parameters| {
                !selected_role(parameters, tcl_registry::ArgRole::ParamList)
            })
            || declaration.site() != body.site()
            || declaration.name_input().policy() != input.policy()
            || !declaration
                .name_input()
                .matches_source(input.source_image(), input.lexer_config())
            || parameters.is_some_and(|parameters| {
                parameters.site() != body.site()
                    || parameters.name_input().policy() != input.policy()
                    || !parameters
                        .name_input()
                        .matches_source(input.source_image(), input.lexer_config())
            })
        {
            return None;
        }
        Some(Self {
            declaration: declaration.clone(),
            body: body.clone(),
            parameters: parameters.cloned(),
            kind,
            span: input.original_word().content_span().ok()?,
            metadata: std::sync::Arc::new(metadata),
        })
    }

    /// Genuine selected source-body role, without an execution-domain grant.
    #[must_use]
    pub const fn kind(&self) -> VendorSourceVariableBodyKind {
        self.kind
    }
    /// Original declaration header word, without display decoding.
    #[must_use]
    pub const fn declaration_name(&self) -> &VendorSourceNameOccurrence {
        &self.declaration
    }
    /// Complete original body word, including source channel and full config.
    #[must_use]
    pub const fn body_input(&self) -> &VendorSourceNameOccurrence {
        &self.body
    }
    /// Original `ParamList` word stays owned even when child units are unavailable.
    #[must_use]
    pub fn parameters(&self) -> Option<&VendorSourceNameOccurrence> {
        self.parameters.as_ref()
    }
    /// Actual script interior; grouping is retained by the body word.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// A source cursor may sit immediately before the body's retained closer.
    /// This is lexical visibility only, without an entered activation grant.
    #[must_use]
    pub fn contains_cursor(&self, offset: u32) -> bool {
        self.span.start() <= offset && offset <= self.span.end()
    }
    /// Actual authored context, Registry generation and selected original roles.
    /// This metadata supplies no reached body or variable storage domain.
    #[must_use]
    pub fn metadata(&self) -> &crate::registry_invocation::VendorRegistryInvocationShape {
        &self.metadata
    }
    /// Complete consumer source/configuration correspondence only.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.body.name_input().matches_source(image, config)
    }
    pub(crate) fn formal_advice(&self) -> Vec<VendorSourceVariableAdvice> {
        let Some(parameters) = self.parameters() else {
            return Vec::new();
        };
        tcl_syntax::naming::vendor_source_formal_extents(
            parameters.name_input().policy(),
            parameters.name_input().original_word(),
        )
        .unwrap_or_default()
        .into_iter()
        .filter_map(|extent| {
            let content = parameters
                .name_input()
                .original_word()
                .content_span()
                .ok()?;
            let start = content
                .start()
                .checked_add(u32::try_from(extent.name.start).ok()?)?;
            let end = content
                .start()
                .checked_add(u32::try_from(extent.name.end).ok()?)?;
            Some(VendorSourceVariableAdvice {
                input: VendorSourceVariableNameInput::Formal {
                    parent: parameters.clone(),
                    extent,
                },
                span: Span::new(start, end),
                form: None,
                home: Some(self.clone()),
                source_root: false,
                metadata: std::sync::Arc::clone(&self.metadata),
            })
        })
        .collect()
    }
}

/// Whole written receiver and original list child are different input kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorSourceVariableNameInput {
    /// Complete original Registry-selected variable operand.
    Written(VendorSourceNameOccurrence),
    /// Original `ParamList` child; no complete word is forged from its name.
    Formal {
        /// Complete original parent parameter-list producer.
        parent: VendorSourceNameOccurrence,
        /// Authenticated child ordinal and source-name geometry.
        extent: VendorSourceFormalExtent,
    },
}

impl VendorSourceVariableNameInput {
    /// Supported source literal units. This supplies no runtime string recipe.
    #[must_use]
    pub fn literal_units(&self) -> Option<&[u8]> {
        match self {
            Self::Written(original) => original
                .name_input()
                .literal_units(VendorSourceNamePurpose::VariableOperand),
            Self::Formal { parent, extent } => parent
                .name_input()
                .literal_units(VendorSourceNamePurpose::FormalList)?
                .get(extent.name.clone()),
        }
    }
    /// The complete original parent word and command layout in either case.
    #[must_use]
    pub const fn original_occurrence(&self) -> &VendorSourceNameOccurrence {
        match self {
            Self::Written(original)
            | Self::Formal {
                parent: original, ..
            } => original,
        }
    }
}

/// Conditional hosted source declaration advice. Naming form and lexical body
/// are retained independently of handler success, alias binding or current cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceVariableAdvice {
    input: VendorSourceVariableNameInput,
    span: Span,
    form: Option<VariableReceiverOperandForm>,
    home: Option<VendorSourceVariableBody>,
    source_root: bool,
    metadata: std::sync::Arc<crate::registry_invocation::VendorRegistryInvocationShape>,
}

impl VendorSourceVariableAdvice {
    pub(crate) fn written(
        original: &VendorSourceNameOccurrence,
        span: Span,
        form: VariableReceiverOperandForm,
        home: Option<&VendorSourceVariableBody>,
        source_root: bool,
        metadata: crate::registry_invocation::VendorRegistryInvocationShape,
    ) -> Option<Self> {
        let words = metadata.original_words();
        let argument = words
            .iter()
            .position(|word| word == original.name_input().original_word())?
            .checked_sub(1)?;
        if !metadata.roles_complete()
            || words != original.original_words()
            || words.iter().any(|word| word.group().expand)
            || metadata.original_head().policy() != original.name_input().policy()
            || (source_root && home.is_some())
            || home.is_some_and(|home| {
                home.metadata().context() != metadata.context()
                    || home.metadata().registry() != metadata.registry()
            })
            || metadata.possible_variable_receiver_operand_form(argument) != Some(form)
            || !metadata.roles().iter().any(|(index, role)| {
                *role == tcl_registry::ArgRole::VarWrite
                    && metadata.argument_offset().checked_add(usize::from(*index)) == Some(argument)
            })
            || metadata.possible_traits().intersects(
                tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
            )
        {
            return None;
        }
        Some(Self {
            input: VendorSourceVariableNameInput::Written(original.clone()),
            span,
            form: Some(form),
            home: home.cloned(),
            source_root,
            metadata: std::sync::Arc::new(metadata),
        })
    }
    /// Actual receiver word or formal-name child extent in the source.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Readonly complete producer or authentic list child, without a native key.
    #[must_use]
    pub const fn input(&self) -> &VendorSourceVariableNameInput {
        &self.input
    }
    /// Supported literal source units; unavailable input stays owned.
    #[must_use]
    pub fn literal_units(&self) -> Option<&[u8]> {
        self.input.literal_units()
    }
    /// Independently selected Registry receiver form; formals are list children.
    #[must_use]
    pub const fn receiver_form(&self) -> Option<VariableReceiverOperandForm> {
        self.form
    }
    /// Original lexical body, without an entered runtime activation.
    #[must_use]
    pub fn home(&self) -> Option<&VendorSourceVariableBody> {
        self.home.as_ref()
    }
    /// The selected advice is at the authentic source root, not an inferred
    /// global variable, `RULE_INIT` broadcast or per-TMM storage declaration.
    #[must_use]
    pub const fn is_source_root(&self) -> bool {
        self.source_root
    }
    /// Actual authored metadata selection, independently of handler success.
    #[must_use]
    pub fn metadata(&self) -> &crate::registry_invocation::VendorRegistryInvocationShape {
        &self.metadata
    }
}

/// Source completion spelling from this genuine supported advice producer.
/// Qualified, array-shaped and unavailable units abstain; no runtime lookup or
/// native object recipe is inferred from a source-card name.
#[must_use]
pub fn vendor_variable_source_reference(advice: &VendorSourceVariableAdvice) -> Option<String> {
    let original = advice.input().original_occurrence().name_input();
    match advice.input() {
        VendorSourceVariableNameInput::Written(_) => {
            (advice.receiver_form() == Some(VariableReceiverOperandForm::Combined)).then_some(())?;
        }
        VendorSourceVariableNameInput::Formal { .. } => {}
    }
    tcl_syntax::naming::vendor_scalar_source_reference(
        original.policy(),
        advice.literal_units()?,
        original.lexer_config(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vendor_advice_survives_report_maps(
        analysis: &mut crate::analyser::AnalysisResult,
        source: &str,
        config: LexerConfig,
        offset: u32,
    ) {
        assert!(analysis.original_variable_symbols.is_empty());
        let saved: Vec<_> = analysis
            .original_vendor_variable_advice()
            .cloned()
            .collect();
        analysis.all_variables.clear();
        analysis.global_scope.variables.clear();
        assert_eq!(
            analysis
                .original_vendor_variable_advice()
                .cloned()
                .collect::<Vec<_>>(),
            saved
        );
        assert!(
            analysis
                .original_vendor_variable_body_in_source(
                    &SourceImage::document(&format!("{source}# changed")),
                    config,
                    offset,
                )
                .is_none()
        );
    }

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn vendor_variable_advice_preserves_selected_body_and_formal_source_owners() {
        let source = "proc p {argument {optional DEFAULT} args} {set local 1; set {v\\uD800} 2; return $argument}\n";
        for profile in ["f5-irules", "f5-iapps", "f5-tmsh"] {
            let mut analysis = crate::analyser::Analyser::new().analyse(source, profile);
            let image = SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
            let body = analysis
                .original_vendor_variable_body_in_source(&image, config, offset)
                .expect(profile)
                .clone();
            assert_eq!(body.kind(), VendorSourceVariableBodyKind::Procedure);
            assert_eq!(
                analysis.original_vendor_variable_body_in_source(&image, config, body.span().end()),
                Some(&body),
                "the cursor before the authentic closing brace remains inside its source body"
            );
            assert!(
                analysis
                    .original_vendor_variable_body_in_source(&image, config, body.span().end() + 1,)
                    .is_none(),
                "the cursor after that closer has left the body"
            );
            assert_eq!(
                body.parameters()
                    .unwrap()
                    .name_input()
                    .literal_units(VendorSourceNamePurpose::FormalList,),
                Some(b"argument {optional DEFAULT} args".as_slice())
            );
            let advice: Vec<_> = analysis.original_vendor_variable_advice().collect();
            for (ordinal, name) in [b"argument".as_slice(), b"optional", b"args"]
                .into_iter()
                .enumerate()
            {
                let formal = advice
                    .iter()
                    .find(|advice| advice.literal_units() == Some(name))
                    .expect(profile);
                let VendorSourceVariableNameInput::Formal { extent, .. } = formal.input() else {
                    panic!("{profile}: original list child required");
                };
                assert_eq!(extent.ordinal, ordinal);
                assert_eq!(extent.has_default, ordinal == 1);
                assert_eq!(extent.is_rest, ordinal == 2);
                assert_eq!(formal.home(), Some(&body));
                assert_eq!(image.bytes().get(formal.span().as_range()), Some(name));
            }
            let local = advice
                .iter()
                .find(|advice| advice.literal_units() == Some(b"local"))
                .expect(profile);
            assert_eq!(local.home(), Some(&body));
            assert_eq!(local.metadata().context(), body.metadata().context());
            assert_eq!(local.metadata().registry(), body.metadata().registry());
            assert_eq!(
                body.metadata().original_words(),
                body.body_input().original_words()
            );
            assert_eq!(
                vendor_variable_source_reference(local).as_deref(),
                Some("${local}")
            );
            assert_eq!(
                local.receiver_form(),
                Some(VariableReceiverOperandForm::Combined)
            );
            let opaque = advice
                .iter()
                .find(|advice| {
                    matches!(advice.input(),
                VendorSourceVariableNameInput::Written(original)
                    if original.name_input().original_word().bytes() == b"{v\\uD800}")
                })
                .expect("unsupported name remains owned");
            assert!(opaque.literal_units().is_none());
            assert!(vendor_variable_source_reference(opaque).is_none());
            assert!(!opaque.is_source_root());
            assert_vendor_advice_survives_report_maps(&mut analysis, source, config, offset);
        }
    }

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn vendor_variable_advice_retains_actual_keyed_context_and_rejects_foreign_roles() {
        use tcl_dialect::model::bigip_execution_context::BigIpExecutionContext;
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let keyed = tcl_registry::model::KeyedVersions {
            bigip: Some(tcl_dialect::model::Version::parse("17.1.0").unwrap()),
            ..Default::default()
        };
        let context = environment
            .context_registry(&keyed, 0)
            .expect("authored keyed hosted context is available");
        let config = LexerConfig::from_grammar(environment.grammar());
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            context.clone(),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse("when HTTP_REQUEST {set local 1}", "presentation-only");
        let advice = analysis
            .original_vendor_variable_advice()
            .find(|advice| advice.literal_units() == Some(b"local".as_slice()))
            .unwrap();
        let axis = tcl_dialect::model::VersionAxisId::package("f5-irules-cmds");
        assert_eq!(
            advice.metadata().context().floors.primary(&axis),
            keyed.bigip.as_ref()
        );
        assert_eq!(
            advice.metadata().registry(),
            &context.commands().snapshot().semantic_key()
        );
        let original = advice.input().original_occurrence();
        let foreign = tcl_registry::model::ingress::static_context_for("f5-tmsh");
        let head = super::super::vendor_name::VendorSourceNameInput::from_original_word(
            &original.original_words()[0],
            original.name_input().policy(),
        );
        assert!(
            crate::registry_invocation::vendor_registry_invocation_shape(
                foreign,
                &head,
                original.original_words(),
            )
            .is_none()
        );
        let explicit = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            context,
            config,
        )
        .with_vendor_source_policy(
            tcl_syntax::naming::VendorSourceNamePolicy::authored(
                BigIpExecutionContext::ICallScript,
            )
            .unwrap(),
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(explicit)
            .analyse("proc p {arg} {set local 1}", "presentation-only");
        assert!(analysis.has_original_vendor_source_names());
        assert!(
            analysis.original_vendor_variable_advice().next().is_none(),
            "a source policy cannot manufacture a different ContextRegistry"
        );
    }

    #[test]
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    fn vendor_event_and_root_source_advice_do_not_select_runtime_domains() {
        let source = "when RULE_INIT {set static::shared 1}\nwhen HTTP_REQUEST {set local 2}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        for name in [b"static::shared".as_slice(), b"local"] {
            let advice = analysis
                .original_vendor_variable_advice()
                .find(|advice| advice.literal_units() == Some(name))
                .expect("event variable advice");
            assert_eq!(
                advice.home().unwrap().kind(),
                VendorSourceVariableBodyKind::Event
            );
            if name == b"static::shared" {
                assert!(
                    vendor_variable_source_reference(advice).is_none(),
                    "no namespace partition donation"
                );
            }
            assert!(!advice.is_source_root());
            assert!(
                analysis
                    .original_vendor_variable_body_in_source(&image, config, advice.span().start())
                    .is_some()
            );
        }
        assert!(analysis.original_variable_symbols.is_empty());
        for profile in ["f5-iapps", "f5-tmsh"] {
            let analysis = crate::analyser::Analyser::new().analyse("set root 1", profile);
            let advice = analysis
                .original_vendor_variable_advice()
                .next()
                .expect(profile);
            assert_eq!(advice.literal_units(), Some(b"root".as_slice()));
            assert!(advice.is_source_root());
            assert!(advice.home().is_none());
        }
        let source = "proc p {n\\uD800} {return}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-iapps");
        let config = analysis.body_lexer_config.unwrap();
        let body = analysis
            .original_vendor_variable_body_in_source(
                &SourceImage::document(source),
                config,
                u32::try_from(source.find("return").unwrap()).unwrap(),
            )
            .unwrap();
        assert!(
            body.parameters().is_some(),
            "unsupported ParamList stays owned"
        );
        assert!(
            analysis
                .original_vendor_variable_advice()
                .all(|advice| !matches!(
                    advice.input(),
                    VendorSourceVariableNameInput::Formal { .. }
                ))
        );
    }
}
