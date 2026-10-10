// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hosted source metadata retains its own context and original word vector.

use std::sync::Arc;

use tcl_lexer::NativeWord;
use tcl_registry::InvocationWord;
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_syntax::naming::VendorSourceNamePurpose;

use crate::signature_scan::vendor_name::VendorSourceNameInput;

/// Authored Registry roles at a genuine hosted source vector.
/// This retains vendor context and original words but supplies no effective
/// native argv, lookup result, shadow closure, Normal or compiler admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorRegistryInvocationShape {
    head: VendorSourceNameInput,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    original: Arc<[NativeWord]>,
    command: String,
    roles: Vec<(u8, tcl_registry::ArgRole)>,
    argument_offset: usize,
    roles_complete: bool,
    possible_traits: tcl_registry::Traits,
    navigation: Option<tcl_registry::source_navigation::SourceNavigationOperand>,
    rule_procedure_operand: Option<usize>,
    possible_receivers: Vec<Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm>>,
    source_transitions: Option<tcl_registry::StateTransitions>,
}

impl VendorRegistryInvocationShape {
    /// Genuine complete command-head word and selected hosted context.
    #[must_use]
    pub const fn original_head(&self) -> &VendorSourceNameInput {
        &self.head
    }

    /// Complete availability context supplied by the source analysis owner.
    #[must_use]
    pub const fn context(&self) -> &ResolvedContext {
        &self.context
    }

    /// Exact command-store content used for this metadata selection.
    #[must_use]
    pub const fn registry(&self) -> &tcl_registry::RegistrySemanticKey {
        &self.registry
    }

    /// Unchanged ordered source words, rather than a materialised argv.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }

    /// Selected authored Registry identity, independently of live presence.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Possible roles relative to the selected member's arguments.
    #[must_use]
    pub fn roles(&self) -> &[(u8, tcl_registry::ArgRole)] {
        &self.roles
    }

    /// Member offset in the source post-head vector.
    #[must_use]
    pub const fn argument_offset(&self) -> usize {
        self.argument_offset
    }

    /// Whether the authored role grammar resolved despite unknown operands.
    #[must_use]
    pub const fn roles_complete(&self) -> bool {
        self.roles_complete
    }

    /// Purpose-selected source navigation operand, without loader capability.
    #[must_use]
    pub const fn source_navigation_operand(
        &self,
    ) -> Option<tcl_registry::source_navigation::SourceNavigationOperand> {
        self.navigation
    }

    /// Possible authored receiver grammar at the source post-head ordinal.
    /// This selects naming shape only; it supplies no live handler, access,
    /// variable storage, Normal or native compiler authority.
    #[must_use]
    pub fn possible_variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm> {
        self.possible_receivers.get(argument).copied().flatten()
    }

    /// Guarded callers may use this original post-head ordinal for a readonly
    /// rule-reference candidate. It supplies neither an owner nor a live call.
    #[must_use]
    pub const fn source_rule_procedure_operand(&self) -> Option<usize> {
        self.rule_procedure_operand
    }

    /// Authored transition intentions at these exact source operands. These
    /// are source-schema barriers only, without execution or commit authority.
    pub(crate) fn source_transitions(&self) -> Option<&tcl_registry::StateTransitions> {
        self.source_transitions.as_ref()
    }

    /// Nominal traits for source assistance, never actual handler effects.
    #[must_use]
    pub const fn possible_traits(&self) -> tcl_registry::Traits {
        self.possible_traits
    }
}

/// Resolve authored metadata from one complete original hosted command vector.
/// The caller supplies the vector retained by its genuine source owner. Every
/// word must belong to the head's complete image and full configuration.
/// Unsupported values retain one original non-expanded word at their ordinals;
/// expansion retains unknown cardinality. No C/Jim string or naming recipe is
/// selected.
#[must_use]
pub fn vendor_registry_invocation_shape(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    original: &[NativeWord],
) -> Option<VendorRegistryInvocationShape> {
    // Implementation contract: naming.vendor.original-registry-metadata
    // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
    if tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
        &context.context().environment.id,
    ) != Some(head.policy().context())
        || original.first()? != head.original_word()
        || original.iter().any(|word| {
            word.image() != head.source_image()
                || word.executable_parts().config() != head.lexer_config()
        })
        || original
            .windows(2)
            .any(|words| words[0].span().end() > words[1].span().start())
    {
        return None;
    }
    // The original input owns the authored coordinate purpose. Compatibility
    // Registry spelling lookup is not the parser for a written Tcl head.
    let coordinate = head.root_command_coordinate()?;
    let command = std::str::from_utf8(coordinate.as_bytes()).ok()?;
    let mut words = vec![InvocationWord::Literal(command)];
    words.extend(original.iter().skip(1).map(|word| {
        if word.group().expand {
            return InvocationWord::Expanded;
        }
        tcl_syntax::naming::vendor_source_literal_units(
            head.policy(),
            word,
            VendorSourceNamePurpose::SourceName,
        )
        .and_then(|value| std::str::from_utf8(value).ok())
        .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
    }));
    let registry = context.commands();
    let selected = tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
        registry,
        Some(context.context()),
        tcl_registry::InvocationWords::structured(words[0], &words[1..]),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let selected = selected.resolved()?;
    let (roles, roles_complete) = selected.authored_source_argument_roles();
    let facts = selected.facts();
    Some(VendorRegistryInvocationShape {
        head: head.clone(),
        context: context.context().clone(),
        registry: registry.snapshot().semantic_key(),
        original: Arc::from(original),
        command: facts.canonical_command.clone(),
        roles,
        argument_offset: facts.argument_offset,
        roles_complete,
        possible_traits: facts.traits,
        source_transitions: selected
            .semantics
            .state_transitions
            .is_declared()
            .then(|| selected.state_transitions()),
        navigation: selected.authored_source_navigation_operand(),
        rule_procedure_operand: selected.authored_source_rule_procedure_operand(),
        possible_receivers: (0..original.len().saturating_sub(1))
            .map(|argument| selected.authored_source_variable_receiver_operand_form(argument))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::bigip_execution_context::BigIpExecutionContext;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::naming::VendorSourceNamePolicy;

    fn words(source: &str, config: LexerConfig) -> Vec<NativeWord> {
        let image = SourceImage::document(source);
        let mut plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(plan.commands.len(), 1);
        plan.commands.remove(0).words
    }

    #[test]
    fn vendor_registry_metadata_retains_context_and_unknown_operand_ordinals() {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        for (profile, hosted) in [
            ("f5-irules", BigIpExecutionContext::TmmIRule),
            ("f5-iapps", BigIpExecutionContext::IAppImplementation),
            ("f5-tmsh", BigIpExecutionContext::TmshCliScript),
        ] {
            let owner = tcl_registry::model::ingress::static_context_for(profile);
            let registry = owner.commands();
            let config = LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            for source in [
                "proc plain {arg} {return $arg}",
                r"proc p\uD800 {arg} {return $arg}",
            ] {
                let original = words(source, config);
                let head = VendorSourceNameInput::from_original_word(
                    &original[0],
                    VendorSourceNamePolicy::authored(hosted).unwrap(),
                );
                let shape = vendor_registry_invocation_shape(owner, &head, &original).unwrap();
                assert_eq!(shape.command(), "proc");
                assert_eq!(shape.original_head().policy().context(), hosted);
                assert_eq!(shape.original_words(), original);
                assert_eq!(shape.argument_offset(), 0);
                assert!(shape.roles_complete());
                assert!(
                    shape
                        .roles()
                        .contains(&(1, tcl_registry::ArgRole::ParamList))
                );
                assert!(shape.roles().contains(&(2, tcl_registry::ArgRole::Body)));
                assert!(
                    shape
                        .original_head()
                        .policy()
                        .observed_variable_input(
                            tcl_syntax::naming::NativeVariableInputForm::Combined(b"arg"),
                            tcl_syntax::naming::ObservedVariableNamePurpose::ScalarReceiver,
                        )
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn vendor_authored_head_coordinates_keep_written_roots_and_source_purpose() {
        // naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        // Source descriptor selection only, not a measured TMM dispatch claim.
        let owner = tcl_registry::model::ingress::static_context_for("f5-irules");
        let config = LexerConfig::from_grammar(owner.commands().profile().unwrap().grammar);
        let policy = VendorSourceNamePolicy::authored(BigIpExecutionContext::TmmIRule).unwrap();
        for source in [
            "matchclass item paths",
            "::matchclass item paths",
            "::::matchclass item paths",
            "{::::matchclass} item paths",
        ] {
            let original = words(source, config);
            let head = VendorSourceNameInput::from_original_word(&original[0], policy);
            let shape = vendor_registry_invocation_shape(owner, &head, &original).unwrap();
            assert_eq!(shape.command(), "matchclass", "{source}");
            assert_eq!(shape.original_words(), original);
            assert_eq!(shape.original_head().original_word(), &original[0]);
            assert_eq!(
                head.root_command_coordinate().unwrap().as_bytes(),
                b"::matchclass"
            );
        }
        for source in ["$command item paths", r"matchclass\ item paths"] {
            let original = words(source, config);
            let head = VendorSourceNameInput::from_original_word(&original[0], policy);
            assert!(head.root_command_coordinate().is_none(), "{source}");
            assert!(vendor_registry_invocation_shape(owner, &head, &original).is_none());
        }
    }

    #[test]
    fn vendor_authored_receivers_retain_opaque_source_operands_and_reject_expansion() {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        use tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
        let owner = tcl_registry::model::ingress::static_context_for("f5-irules");
        let config = LexerConfig::from_grammar(owner.commands().profile().unwrap().grammar);
        let policy = VendorSourceNamePolicy::authored(BigIpExecutionContext::TmmIRule).unwrap();
        for (source, expected) in [
            (r"set name\uD800 VALUE", Some(Combined)),
            ("set $name VALUE", Some(Combined)),
            ("set", None),
            ("set {*}$arguments", Some(Combined)),
            ("global name", None),
        ] {
            let original = words(source, config);
            let head = VendorSourceNameInput::from_original_word(&original[0], policy);
            let shape = vendor_registry_invocation_shape(owner, &head, &original).unwrap();
            assert_eq!(
                shape.possible_variable_receiver_operand_form(0),
                expected,
                "{source}"
            );
        }
        // The measured hosted grammar keeps the asterisk and variable in
        // separate words without expansion.
        assert!(!config.expand_syntax);
        let original = words("set {*}$arguments", config);
        assert_eq!(original.len(), 3);
        assert!(!original[1].group().expand);
        assert!(!original[2].group().expand);
        let head = VendorSourceNameInput::from_original_word(&original[0], policy);
        let shape = vendor_registry_invocation_shape(owner, &head, &original).unwrap();
        assert_eq!(shape.original_words(), original);

        // An explicitly expansion-enabled source retains unknown cardinality.
        // This is a custom source configuration, not a hosted runtime claim.
        let expanded_config = LexerConfig {
            expand_syntax: true,
            ..config
        };
        let original = words("set {*}$arguments", expanded_config);
        assert_eq!(original.len(), 2);
        assert!(original[1].group().expand);
        let head = VendorSourceNameInput::from_original_word(&original[0], policy);
        let shape = vendor_registry_invocation_shape(owner, &head, &original).unwrap();
        assert!(shape.possible_variable_receiver_operand_form(0).is_none());
    }

    #[test]
    fn vendor_registry_metadata_requires_matching_hosted_context_and_retains_keyed_axes() {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let keyed = tcl_registry::model::KeyedVersions {
            bigip: Some(tcl_dialect::model::Version::parse("17.1.0").unwrap()),
            ..Default::default()
        };
        let owner = environment
            .context_registry(&keyed, 0)
            .expect("authored keyed hosted context is available");
        let config = LexerConfig::from_grammar(owner.commands().profile().unwrap().grammar);
        let original = words("proc name {} {}", config);
        let policy = VendorSourceNamePolicy::authored(BigIpExecutionContext::TmmIRule).unwrap();
        let head = VendorSourceNameInput::from_original_word(&original[0], policy);
        let shape = vendor_registry_invocation_shape(&owner, &head, &original).unwrap();
        let axis = tcl_dialect::model::VersionAxisId::package("f5-irules-cmds");
        assert_eq!(shape.context().floors.primary(&axis), keyed.bigip.as_ref());
        assert_eq!(
            *shape.registry(),
            owner.commands().snapshot().semantic_key()
        );
        assert_eq!(shape, shape.clone());
        let default_owner = environment.default_context_registry();
        let default_shape =
            vendor_registry_invocation_shape(&default_owner, &head, &original).unwrap();
        assert_eq!(shape.command(), default_shape.command());
        assert_eq!(shape.original_head(), default_shape.original_head());
        assert_ne!(
            shape, default_shape,
            "complete keyed contexts remain different"
        );
        for profile in ["f5-iapps", "f5-tmsh", "tcl8.6"] {
            let foreign = tcl_registry::model::ingress::static_context_for(profile);
            assert!(vendor_registry_invocation_shape(foreign, &head, &original).is_none());
        }
        let icall = VendorSourceNameInput::from_original_word(
            &original[0],
            VendorSourceNamePolicy::authored(BigIpExecutionContext::ICallScript).unwrap(),
        );
        assert!(vendor_registry_invocation_shape(&owner, &icall, &original).is_none());
    }

    #[test]
    fn vendor_registry_metadata_declines_dynamic_heads_and_foreign_words() {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let owner = tcl_registry::model::ingress::static_context_for("f5-irules");
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let context = owner;
        let config = LexerConfig::from_grammar(profile.grammar);
        let policy = VendorSourceNamePolicy::authored(BigIpExecutionContext::TmmIRule).unwrap();
        for source in ["$selected NAME", r"pr\u006fc NAME"] {
            let original = words(source, config);
            let head = VendorSourceNameInput::from_original_word(&original[0], policy);
            assert!(vendor_registry_invocation_shape(context, &head, &original).is_none());
        }
        let original = words("proc name {} {}", config);
        let head = VendorSourceNameInput::from_original_word(&original[0], policy);
        let foreign = words("proc other {} {}", config);
        assert!(vendor_registry_invocation_shape(context, &head, &foreign).is_none());
        let mut override_config = config;
        override_config.strict_quoting = !override_config.strict_quoting;
        let foreign = words("proc name {} {}", override_config);
        assert!(vendor_registry_invocation_shape(context, &head, &foreign).is_none());
    }
}
