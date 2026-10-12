// SPDX-License-Identifier: AGPL-3.0-or-later
//! Current command selection for a proposed scalar assignment.
//!
//! An inserted command is authored output, not an original executed operand.
//! Its selected Registry operation, current occupied slot and original source
//! point are retained independently of receiver absence and store completion.

use super::{
    Arc, BindingKind, CommandAllocationSite, CommandIdentity, MayBinding, SourceCommandKey,
    SourceInvocationBinding, SourceLookupSnapshot, SourceNamespaceKey,
};
use crate::signature_scan::scope::SignatureNamespaceScope;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::LexerConfig;
use tcl_registry::{CommandRegistry, InvocationWord, InvocationWords, SemanticOperationId};
use tcl_syntax::naming::NamePolicyProtocol;

/// A callable authored command word selecting the current scalar setter at
/// one original source point. This supplies neither receiver availability,
/// normal completion, value lifetime, edit placement nor native preparation.
#[derive(Debug, Clone)]
pub struct OriginalScalarAssignmentCommand {
    source_word: String,
    slot: ByteCommandSlot,
    site: CommandAllocationSite,
    config: LexerConfig,
    policy: NamePolicyProtocol,
    identity: CommandIdentity,
    snapshot: Arc<SourceLookupSnapshot>,
}

impl OriginalScalarAssignmentCommand {
    /// One literal command word under the retained channel and full grammar.
    #[must_use]
    pub fn source_word(&self) -> &str {
        &self.source_word
    }

    /// Exact callable slot, independent of its source spelling.
    #[must_use]
    pub const fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }

    /// Original operation point at which current selection was checked.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }

    /// Full original parser configuration, without a new compiler grant.
    #[must_use]
    pub const fn lexer_config(&self) -> LexerConfig {
        self.config
    }

    /// Independently selected engine naming purpose and authority.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// The same retained point and selected implementation, without looking
    /// up a completed module world or a reconstructed original command word.
    #[must_use]
    pub fn matches_original_point(&self, binding: &SourceInvocationBinding) -> bool {
        binding.invocation_site() == Some(&self.site)
            && binding.lookup_state.as_ref().is_some_and(|snapshot| {
                snapshot.as_ref() == self.snapshot.as_ref()
                    && !snapshot
                        .state
                        .source_execution_observed(Some(&self.identity))
                    && !snapshot
                        .state
                        .runtime_execution_observed(Some(&self.identity))
            })
    }
}

impl SourceInvocationBinding {
    /// Select the authored scalar-assignment command at this original reached
    /// point. The original vector owns syntax; it is never replaced by the
    /// proposed command. Variable/store effects require separate receipts.
    #[must_use]
    pub fn original_scalar_assignment_command(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalScalarAssignmentCommand> {
        if tokens.source_binding.as_ref() != Some(self)
            || self.runtime_reachability() != super::SourceRuntimeReachability::Reached
            || self.entered_execution_observer.observed()
        {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        self.lookup_state
            .as_ref()?
            .original_scalar_assignment_template(
                self.invocation_site()?,
                &self.lookup_namespace_key,
                config,
                registry,
            )
    }
}

impl SourceLookupSnapshot {
    fn original_scalar_assignment_template(
        self: &Arc<Self>,
        site: &CommandAllocationSite,
        namespace: &SourceNamespaceKey,
        config: LexerConfig,
        registry: &CommandRegistry,
    ) -> Option<OriginalScalarAssignmentCommand> {
        let state = &self.state;
        if state.current_source_origin.as_ref() != Some(&site.source)
            || state.has_opaque_domain()
            || state.baseline.unknown_entry
            || state.source_step_observed()
            || matches!(
                state.variable_frame,
                crate::var_resolve::VariableExecutionFrame::Unknown
            )
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
        {
            return None;
        }
        let dialect = state.baseline.dialect?;
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        if state
            .source_variables
            .execution_name_policy?
            .native_recipe()?
            != policy
            || !state
                .source_variables
                .invocation_dialect?
                .has_same_execution_policy(dialect)
            || config.grammar_over(dialect.lexer_grammar) != dialect.lexer_grammar
        {
            return None;
        }
        state.original_namespace_geometry(namespace, policy)?;

        // These are explicitly authored Registry descriptors. No dynamic value
        // is fabricated for the two proposed operands; this selects layout only.
        let operation =
            SemanticOperationId::StructuredLowering(tcl_registry::hooks::LoweringHookId::Set);
        let mut descriptors = registry
            .command_names_for_semantic_operation(operation)
            .filter_map(|name| registry.get_for_surface(name, dialect.authoring_query()))
            .filter(|spec| spec.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Set));
        let expected = descriptors.next()?;
        if descriptors.next().is_some() {
            return None;
        }
        let proposed = [InvocationWord::Dynamic, InvocationWord::Dynamic];
        let resolution = registry.resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(expected.name), &proposed)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = resolution.resolved()?.facts();
        if facts.operation != operation
            || facts.native_result
                != Some(
                    tcl_registry::native_result::NativeResultContract::VariableValue {
                        variable_at: 0,
                        phase: tcl_registry::native_result::VariableResultPhase::AfterWrite,
                    },
                )
        {
            return None;
        }
        let target = state.original_registry_metadata_target(namespace, expected.name, policy)?;
        if target.kind != BindingKind::Builtin
            || target.implementation_generation != 0
            || target.implementation_allocation.is_some()
            || !registry
                .get_for_surface(&target.command, dialect.authoring_query())
                .is_some_and(|actual| std::ptr::eq(actual, expected))
            || state.source_execution_observed(target.token.as_ref())
            || state.runtime_execution_observed(target.token.as_ref())
        {
            return None;
        }
        let identity = target.token?;

        let slot = self.original_scalar_assignment_slot(namespace, expected.name, policy)?;
        let source_word = tcl_syntax::naming::native_command_source_word(
            policy.recipe(),
            &slot,
            site.source.source_image().channel(),
            config,
        )?;
        Some(OriginalScalarAssignmentCommand {
            source_word,
            slot,
            site: site.clone(),
            config,
            policy,
            identity,
            snapshot: Arc::clone(self),
        })
    }
    fn original_scalar_assignment_slot(
        &self,
        namespace: &SourceNamespaceKey,
        name: &str,
        policy: NamePolicyProtocol,
    ) -> Option<ByteCommandSlot> {
        let state = &self.state;
        // The central target selector has checked every implementation. This
        // separate projection retains the unanimously occupied callable slot.
        let mut unanimous = None;
        for path in state.original_registry_command_paths(namespace, name, policy)? {
            let mut selected = None;
            for key in path {
                let bindings = state.original_bindings_for_key(&key)?;
                if bindings.is_empty() || bindings.contains(&MayBinding::Unknown) {
                    return None;
                }
                if bindings
                    .iter()
                    .all(|binding| *binding == MayBinding::Missing)
                {
                    continue;
                }
                if bindings.contains(&MayBinding::Missing) {
                    return None;
                }
                selected = Some(key);
                break;
            }
            let key = selected?;
            let SourceCommandKey::Slot { namespace, simple } = key else {
                return None;
            };
            let holder = match state.original_namespace_geometry(&namespace, policy)? {
                SignatureNamespaceScope::C(path) => path,
                SignatureNamespaceScope::Jim(_) => tcl_core_types::ByteNamespacePath::root(),
                SignatureNamespaceScope::Symbolic(_) => return None,
            };
            let slot = ByteCommandSlot::new(holder, simple);
            if unanimous.as_ref().is_some_and(|previous| previous != &slot) {
                return None;
            }
            unanimous = Some(slot);
        }
        unanimous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        environment: &str,
    ) -> (
        Arc<SourceLookupSnapshot>,
        CommandAllocationSite,
        LexerConfig,
    ) {
        let context = tcl_registry::model::ingress::static_context_for(environment);
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
        );
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let mut state = super::super::ModuleCommandBindings::initial_with_options(
            context.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(config),
        );
        let origin = Arc::new(super::super::SourceOriginId::authored_image(
            tcl_lexer::SourceImage::document("list VALUE"),
        ));
        state.current_source_origin = Some(Arc::clone(&origin));
        let site = CommandAllocationSite {
            source: origin,
            offset: 0,
        };
        (Arc::new(SourceLookupSnapshot::new(state)), site, config)
    }

    #[test]
    fn proposed_assignment_retains_current_registry_slot_and_source_point() {
        // Implementation contract: naming.compiler.current-assignment-command-template
        // docs/design/analysis/name-resolution-proofs/current-assignment-command-template.md
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            let (snapshot, site, config) = snapshot(environment);
            let namespace = snapshot.state.source_root_namespace_key().unwrap();
            let receipt = snapshot
                .original_scalar_assignment_template(&site, &namespace, config, registry)
                .unwrap();
            assert_eq!(receipt.site(), &site);
            assert_eq!(receipt.lexer_config(), config);
            assert_eq!(receipt.slot().simple.as_bytes(), b"set");
            assert!(!receipt.source_word().is_empty());
            let mut foreign_config = config;
            foreign_config.expand_syntax = !foreign_config.expand_syntax;
            assert!(
                snapshot
                    .original_scalar_assignment_template(
                        &site,
                        &namespace,
                        foreign_config,
                        registry
                    )
                    .is_none()
            );
            let foreign = CommandAllocationSite {
                source: Arc::new(super::super::SourceOriginId::authored_image(
                    tcl_lexer::SourceImage::document("list OTHER"),
                )),
                offset: 0,
            };
            assert!(
                snapshot
                    .original_scalar_assignment_template(&foreign, &namespace, config, registry)
                    .is_none()
            );
            let foreign_registry =
                tcl_registry::model::ingress::static_context_for(if environment == "tcl8.4" {
                    "jim"
                } else {
                    "tcl8.4"
                })
                .commands();
            assert!(
                snapshot
                    .original_scalar_assignment_template(
                        &site,
                        &namespace,
                        config,
                        foreign_registry
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn proposed_assignment_declines_missing_shadowed_wrapped_and_unknown_setters() {
        // Implementation contract: naming.compiler.current-assignment-command-template
        // docs/design/analysis/name-resolution-proofs/current-assignment-command-template.md
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            let (snapshot, site, config) = snapshot(environment);
            let namespace = snapshot.state.source_root_namespace_key().unwrap();
            let policy = snapshot
                .state
                .baseline
                .execution_name_policy
                .unwrap()
                .native_recipe()
                .unwrap();
            let key = snapshot
                .state
                .original_registry_command_paths(&namespace, "set", policy)
                .unwrap()[0][0]
                .clone();
            let target = snapshot
                .state
                .original_registry_metadata_target(&namespace, "set", policy)
                .unwrap();
            let mut shadowed = target.clone();
            shadowed.kind = BindingKind::Proc;
            shadowed.registry_backed = false;
            let mut wrapped = target;
            wrapped.kind = BindingKind::Alias;
            for replacement in [
                MayBinding::Missing,
                MayBinding::Unknown,
                MayBinding::Target(shadowed),
                MayBinding::Target(wrapped),
            ] {
                let mut changed = snapshot.state.clone();
                changed.replace(key.clone(), super::super::BTreeSet::from([replacement]));
                let changed = Arc::new(SourceLookupSnapshot::new(changed));
                assert!(
                    changed
                        .original_scalar_assignment_template(&site, &namespace, config, registry)
                        .is_none(),
                    "{environment}"
                );
            }
        }
    }
}
