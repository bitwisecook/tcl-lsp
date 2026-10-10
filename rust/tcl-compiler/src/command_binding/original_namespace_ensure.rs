// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Same-operation namespace allocation correspondence for variable transfer.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceExecutionContext, SourceNamespaceKey,
    SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{
    InvocationFacts, NamespaceTransition, NamespaceTransitionTarget, StateTransition,
    TransitionSubject,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalNamespaceEnsureOperand {
    subject: TransitionSubject,
    input: SignatureSourceNameInput,
}

/// Genuine namespace operands before the selected command transfer. Retention
/// alone grants neither allocation, completion nor native namespace authority.
pub(super) struct OriginalNamespaceEnsureTransfer {
    site: CommandAllocationSite,
    current: SourceNamespaceKey,
    operands: Vec<OriginalNamespaceEnsureOperand>,
}

/// Actual namespace identities reached by the same canonical command transfer.
pub(crate) struct AppliedOriginalNamespaceEnsures {
    site: CommandAllocationSite,
    namespaces: Vec<(OriginalNamespaceEnsureOperand, SourceNamespaceKey)>,
}

/// Independent allocation and retirement handoffs for one variable transfer.
#[derive(Clone, Copy, Default)]
pub(crate) struct OriginalNamespaceCellOperations<'a> {
    pub(crate) ensures: Option<&'a AppliedOriginalNamespaceEnsures>,
    pub(crate) deletions:
        Option<&'a super::original_namespace_deletion::AppliedOriginalNamespaceDeletions>,
}

impl OriginalNamespaceEnsureTransfer {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::NamespaceEval,
            )
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval)
            || !facts.arg_roles_complete
        {
            return None;
        }
        let transitions = facts.state_transitions.declared()?;
        let [fact] = transitions.facts() else {
            return None;
        };
        let StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(subject),
        }) = &fact.transition
        else {
            return None;
        };
        if !facts.arg_roles.iter().any(|(index, role)| {
            *role == tcl_registry::ArgRole::NamespaceName
                && facts.argument_offset.checked_add(usize::from(*index))
                    == subject.argument_index()
        }) {
            return None;
        }
        let input =
            super::original_command_table::original_operand(native, subject, state, context);
        let input = input?;
        let operands = vec![OriginalNamespaceEnsureOperand {
            subject: subject.clone(),
            input,
        }];
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            current: context.namespace_identity(),
            operands,
        })
    }

    pub(super) fn after_command_transfer(
        self,
        state: &ModuleCommandBindings,
    ) -> Option<AppliedOriginalNamespaceEnsures> {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        if state.has_opaque_domain()
            || state.current_source_origin.as_ref() != Some(&self.site.source)
        {
            return None;
        }
        let namespaces = self
            .operands
            .into_iter()
            .map(|operand| {
                let namespace =
                    state.original_namespace_key_for_input(&self.current, &operand.input);
                let namespace = namespace?;
                state
                    .namespaces
                    .contains(&namespace)
                    .then_some((operand, namespace))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(AppliedOriginalNamespaceEnsures {
            site: self.site,
            namespaces,
        })
    }
}

impl AppliedOriginalNamespaceEnsures {
    pub(crate) fn namespace(
        &self,
        subject: &TransitionSubject,
        operands: Option<&crate::variable_bindings::OriginalVariableInvocation>,
        state: &crate::var_resolve::ResolveContext,
        offset: u32,
    ) -> Option<&SourceNamespaceKey> {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        if self.site.offset != offset {
            return None;
        }
        self.namespaces.iter().find_map(|(operand, namespace)| {
            (operand.subject == *subject
                && state.namespace_identities.contains(namespace)
                && subject
                    .argument_index()
                    .and_then(|index| operands?.input(index, state))
                    == Some(&operand.input))
            .then_some(namespace)
        })
    }
}

#[cfg(test)]
mod tests {
    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let owner = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            owner.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_namespace_ensure_shares_actual_allocations_without_widening_empty_cells() {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"namespace eval N\uD800 {namespace eval child {proc p\uD801 {} {}}}";
            let bindings = analyse(source, version);
            let state = &bindings.final_state;
            let variables = &state.source_variables;
            assert!(variables.namespace_cells.closed, "{version:?}");
            assert!(!variables.dynamic_bindings, "{version:?}");
            assert!(!variables.dynamic_traces, "{version:?}");
            for segments in [
                vec![b"N\xed\xa0\x80".as_slice()],
                vec![b"N\xed\xa0\x80".as_slice(), b"child".as_slice()],
            ] {
                let namespace = state
                    .namespaces
                    .iter()
                    .find(|namespace| {
                        namespace.exact_native_path().is_some_and(|path| {
                            path.as_segments()
                                .iter()
                                .map(tcl_core_types::NameBytes::as_bytes)
                                .collect::<Vec<_>>()
                                == segments
                        })
                    })
                    .unwrap_or_else(|| panic!("{version:?}: actual allocation"));
                assert!(
                    variables
                        .namespace_addressable_identities
                        .contains(namespace),
                    "{version:?}"
                );
            }
            assert_eq!(
                bindings
                    .original_completed_command_world()
                    .unwrap()
                    .declarations()
                    .count(),
                1
            );
        }
    }

    #[test]
    fn original_namespace_ensure_does_not_close_unknown_operands_or_body_observers() {
        // naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "namespace eval $unknown {proc p {} {}}",
                r"namespace eval N\uD800 {unknown_callback}",
                r"namespace eval N\uD800 $unknown_body",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
    }
}
