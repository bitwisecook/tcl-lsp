// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One conditional Registry schema owner for source cards and variable advice.

use crate::command_binding::{OriginalCatalogueSourceCandidate, OriginalCatalogueSourceObligation};
use crate::ir::CommandTokens;
use crate::signature_scan::original_name::SourceOriginalNameOccurrence;
use crate::signature_scan::scope::SignatureNamespaceScope;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_registry::{ArgRole, InvocationWord, Traits};

/// Registry-authored source schema with independently retained applicability
/// obligations. Roles describe conditional source syntax only; this grants no
/// execution, entered frame, command publication, cell, CPP, Normal or edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalConditionalRegistryMetadata {
    catalogue: OriginalCatalogueSourceCandidate,
    context: ResolvedContext,
    command: String,
    roles: Vec<(u8, ArgRole)>,
    argument_offset: usize,
    roles_complete: bool,
    traits: Traits,
    receivers: Vec<Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm>>,
    symbol: Option<tcl_registry::SymbolDef>,
}
impl OriginalConditionalRegistryMetadata {
    /// Complete genuine original source head and site.
    #[must_use]
    pub const fn original_head(&self) -> &SourceOriginalNameOccurrence {
        self.catalogue.original_head()
    }
    /// Complete unchanged original native word vector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        self.catalogue.original_words()
    }
    /// Independently retained original source grammar, without compiler admission.
    #[must_use]
    pub fn original_source_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.catalogue.original_source_dialect()
    }

    /// Source namespace geometry, independent of executing holders.
    #[must_use]
    pub const fn original_namespace(&self) -> &SignatureNamespaceScope {
        self.catalogue.original_namespace()
    }
    /// Actual full analysis availability and package context.
    #[must_use]
    pub const fn context(&self) -> &ResolvedContext {
        &self.context
    }
    /// Canonical authored Registry descriptor; no runtime target is supplied.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Selected authored source roles, with original post-head ordinals.
    #[must_use]
    pub fn roles(&self) -> &[(u8, ArgRole)] {
        &self.roles
    }
    /// Whether the authored source-role grammar is determinate.
    #[must_use]
    pub const fn roles_complete(&self) -> bool {
        self.roles_complete
    }
    /// Selected member's offset in the unchanged post-head vector.
    #[must_use]
    pub const fn argument_offset(&self) -> usize {
        self.argument_offset
    }
    /// Nominal source-schema traits, without an actual effect assertion.
    #[must_use]
    pub const fn possible_traits(&self) -> Traits {
        self.traits
    }
    /// Conditional receiver form at one exact original post-head ordinal.
    #[must_use]
    pub fn possible_variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm> {
        self.receivers.get(argument).copied().flatten()
    }
    /// The independently selected source `SymbolDef` descriptor, if applicable.
    #[must_use]
    pub const fn symbol_definition(&self) -> Option<tcl_registry::SymbolDef> {
        self.symbol
    }
    /// Unknown or alternative applicability remains explicit on this schema.
    #[must_use]
    pub fn obligations(&self) -> &[OriginalCatalogueSourceObligation] {
        self.catalogue.obligations()
    }
    /// Exact source/channel and full grammar correspondence only.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.catalogue.matches_source(image, config)
    }
    /// Same actual command-store contents, independent of presentation labels.
    #[must_use]
    pub fn matches_registry(&self, registry: &tcl_registry::CommandRegistry) -> bool {
        self.catalogue.matches_registry(registry)
    }
}

/// Conditional source schema from a genuine original source/vector, actual
/// context, byte catalogue coordinates and independent shadow barriers.
/// Missing original owners never fall back to a reporting command spelling.
#[must_use]
pub fn original_conditional_registry_metadata(
    context: &ContextRegistry,
    tokens: &CommandTokens,
    head: &SourceOriginalNameOccurrence,
    namespace: Option<&SignatureNamespaceScope>,
) -> Option<OriginalConditionalRegistryMetadata> {
    let registry = context.commands();
    let catalogue = tokens
        .source_binding
        .as_ref()?
        .original_catalogue_source_candidate(tokens, head, namespace, registry)?;
    original_catalogue_registry_metadata(context, catalogue)
}

pub(crate) fn original_catalogue_registry_metadata(
    context: &ContextRegistry,
    catalogue: OriginalCatalogueSourceCandidate,
) -> Option<OriginalConditionalRegistryMetadata> {
    let registry = context.commands();
    let head = catalogue.original_head();
    context
        .context()
        .resolve_spec(registry, &catalogue.candidate().name)?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        catalogue.original_words(),
        head.name_input().policy().string_protocol(),
    )
    .ok()?;
    let mut words = vec![InvocationWord::Literal(&catalogue.candidate().name)];
    words.extend(
        catalogue
            .original_words()
            .iter()
            .enumerate()
            .skip(1)
            .map(|(ordinal, word)| {
                if word.group().expand {
                    InvocationWord::Expanded
                } else {
                    super::source_structure::source_schema_word(
                        captured
                            .literal(ordinal)
                            .map_or(InvocationWord::Dynamic, InvocationWord::KnownBytes),
                    )
                }
            }),
    );
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            Some(context.context()),
            tcl_registry::InvocationWords::structured(words[0], &words[1..]),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
    let selected = resolution.resolved()?;
    let (roles, roles_complete) = selected.authored_source_argument_roles();
    let facts = selected.facts();
    let symbol = registry
        .defines_symbol(
            &facts.canonical_command,
            Some(context.context().authoring_query()),
        )
        .copied();
    Some(OriginalConditionalRegistryMetadata {
        context: context.context().clone(),
        command: facts.canonical_command,
        roles,
        roles_complete,
        argument_offset: facts.argument_offset,
        traits: facts.traits,
        receivers: (0..catalogue.original_words().len().saturating_sub(1))
            .map(|ordinal| selected.authored_source_variable_receiver_operand_form(ordinal))
            .collect(),
        symbol,
        catalogue,
    })
}

impl crate::analyser::AnalysisResult {
    /// Readonly conditional Registry syntax at an actual original command
    /// head. This uses the retained pre-walk source/vector and schema; stale
    /// images, config changes, conflicts and blocked candidates abstain.
    /// Applicability obligations remain on the metadata and supply no runtime
    /// lookup, entered frame, cell, Normal, compiler or edit permission.
    #[must_use]
    pub fn original_conditional_registry_metadata_in_source(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        head_offset: u32,
    ) -> Option<&OriginalConditionalRegistryMetadata> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let metadata = self
            .original_conditional_registry_metadata
            .get(&head_offset)?
            .as_ref()?;
        let context = self.resolved_input.as_ref()?.context_registry();
        (metadata.matches_source(image, config)
            && metadata.matches_registry(context.commands())
            && metadata.context() == context.context()
            && metadata.original_words().first().is_some_and(|word| {
                word.span().start() == head_offset
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span.start() == head_offset)
            }))
        .then_some(metadata)
    }
}

#[cfg(test)]
mod source_inventory_tests {
    use super::*;

    #[test]
    fn original_conditional_source_inventory_preserves_roles_and_current_owners() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "set first 1\nset second 2\n";
        let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        analysis.command_invocations.clear();
        analysis.global_scope.variables.clear();
        let offset = u32::try_from(source.find("set second").unwrap()).unwrap();
        let metadata = analysis
            .original_conditional_registry_metadata_in_source(&image, config, offset)
            .unwrap();
        assert_eq!(metadata.original_words().len(), 3);
        assert!(metadata.roles_complete());
        assert!(metadata.roles().contains(&(0, ArgRole::VarWrite)));
        assert_eq!(metadata.original_words()[1].span().start(), offset + 4);
        let stale = SourceImage::document(&source.replace("second", "absent"));
        assert!(
            analysis
                .original_conditional_registry_metadata_in_source(&stale, config, offset)
                .is_none()
        );
        assert!(
            analysis
                .original_conditional_registry_metadata_in_source(
                    &image,
                    tcl_lexer::LexerConfig {
                        expand_syntax: !config.expand_syntax,
                        ..config
                    },
                    offset,
                )
                .is_none()
        );

        for source in [
            "proc set {args} {}\nset second 2\n",
            "rename set {}\nset second 2\n",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("set second").unwrap()).unwrap();
            assert!(
                analysis
                    .original_conditional_registry_metadata_in_source(
                        &SourceImage::document(source),
                        analysis.body_lexer_config.unwrap(),
                        offset,
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn original_conditional_source_controls_select_subcommands_without_reencoding_payloads() {
        // Implementation contract: naming.variable.conditional-registry-receiver-geometry
        // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
        use tcl_registry::resolved_invocation::VariableReceiverOperandForm;

        let source = concat!(
            "namespace eval n\\uD800 {}\n",
            "namespace exists n\\uD800\n",
            "namespace eval N {}\n",
            "set ::N::v\\uD800 1\n",
            "info exists ::N::v\\uD800\n",
        );
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let image = SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("info exists").unwrap()).unwrap();
            let metadata = analysis
                .original_conditional_registry_metadata_in_source(&image, config, offset)
                .expect(dialect);
            assert!(metadata.roles_complete(), "{dialect}");
            assert_eq!(metadata.argument_offset(), 1, "{dialect}");
            assert!(
                metadata.roles().contains(&(0, ArgRole::VarRead)),
                "{dialect}"
            );
            assert_eq!(
                metadata.possible_variable_receiver_operand_form(1),
                Some(VariableReceiverOperandForm::Combined),
                "{dialect}",
            );
            let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                metadata.original_words(),
                metadata
                    .original_head()
                    .name_input()
                    .policy()
                    .string_protocol(),
            )
            .unwrap();
            assert_eq!(captured.literal(1), Some(b"exists".as_slice()));
            assert_eq!(captured.literal(2), Some(b"::N::v\xed\xa0\x80".as_slice()));
            assert!(
                analysis
                    .original_conditional_registry_metadata_in_source(
                        &SourceImage::document(&format!("{source} ")),
                        config,
                        offset,
                    )
                    .is_none(),
            );
        }

        let raw_zero = super::super::source_structure::source_schema_word(
            InvocationWord::KnownBytes(b"exists\0tail"),
        );
        assert_eq!(raw_zero, InvocationWord::KnownBytes(b"exists\0tail"));
        let opaque = super::super::source_structure::source_schema_word(
            InvocationWord::KnownBytes(b"exists\xed\xa0\x80"),
        );
        assert_eq!(opaque, InvocationWord::KnownBytes(b"exists\xed\xa0\x80"));
        for word in [
            InvocationWord::Dynamic,
            InvocationWord::Expanded,
            InvocationWord::Opaque,
        ] {
            assert_eq!(
                super::super::source_structure::source_schema_word(word),
                word
            );
        }
        let registry = tcl_registry::CommandRegistry::build_default();
        let arguments = [raw_zero, InvocationWord::KnownBytes(b"::N::v\xed\xa0\x80")];
        let selection = registry
            .resolve_structured_invocation(
                tcl_registry::InvocationWords::structured(
                    InvocationWord::Literal("info"),
                    &arguments,
                ),
                None,
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selection.authored_source_variable_receiver_operand_form(1),
            None
        );
    }
}
