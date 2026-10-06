// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual normal physical worlds after interpreter-owned captured stores.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings,
    SourceInvocationBinding, SourceNativeInvocation, SourceOriginId, SourceOutcomes,
    SourceRuntimeReachability,
};
use crate::var_resolve::ResolveContext;
use std::collections::BTreeMap;

pub(super) type NormalVariableContinuations =
    BTreeMap<CommandAllocationSite, Option<Arc<ResolveContext>>>;

impl SourceInvocationBinding {
    /// Actual normal variable world after a captured native store and its
    /// variable callbacks. Missing evidence grants no successful-store proof.
    #[must_use]
    pub fn normal_variable_continuation(&self) -> Option<&ResolveContext> {
        self.normal_variable_continuation.as_deref()
    }

    pub(super) fn join_normal_variable_continuation(&mut self, other: &Self) {
        match (
            &mut self.normal_variable_continuation,
            &other.normal_variable_continuation,
        ) {
            (Some(left), Some(right)) => Arc::make_mut(left).join(right),
            _ => self.normal_variable_continuation = None,
        }
    }
}

impl SourceCommandBindings {
    pub(super) fn retain_captured_normal_variables(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        before: &ModuleCommandBindings,
        outcomes: &SourceOutcomes,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let writes = crate::variable_bindings::source_variable_write_places(
            facts,
            native.invocation.arguments(),
            &before.source_variables,
            registry,
        );
        if writes.is_empty()
            || (!writes.iter().any(|place| place.observed)
                && !crate::variable_bindings::source_variable_read_places(
                    facts,
                    native.invocation.arguments(),
                    &before.source_variables,
                    registry,
                )
                .iter()
                .any(|place| place.observed))
        {
            return;
        }
        let Some(origin) = &before.current_source_origin else {
            return;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.segment.span.start(),
        };
        let continuation = unobserved_command_boundary(before, native)
            .then(|| {
                outcomes
                    .normal
                    .as_ref()
                    .map(|normal| Arc::clone(&normal.source_variables))
            })
            .flatten();
        self.normal_variable_continuations
            .entry(site)
            .and_modify(|old| match (old.as_mut(), continuation.as_ref()) {
                (Some(old), Some(incoming)) => Arc::make_mut(old).join(incoming),
                _ => *old = None,
            })
            .or_insert(continuation);
    }

    /// Join the complete normal dispatch, including every live alternative and
    /// command exit callback, before the receipt becomes queryable.
    pub(super) fn complete_normal_variable_continuation(
        &mut self,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
        outcomes: &SourceOutcomes,
        observed_boundary: bool,
    ) {
        let Some(origin) = origin else {
            return;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let Some(retained) = self.normal_variable_continuations.get_mut(&site) else {
            return;
        };
        if observed_boundary {
            *retained = None;
        } else if let (Some(retained), Some(normal)) = (retained.as_mut(), outcomes.normal.as_ref())
        {
            Arc::make_mut(retained).join(&normal.source_variables);
        } else {
            *retained = None;
        }
    }

    pub(super) fn attach_normal_variable_continuation(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        if binding.runtime_reachability() == SourceRuntimeReachability::Reached {
            binding.normal_variable_continuation = origin.and_then(|origin| {
                self.normal_variable_continuations
                    .get(&CommandAllocationSite {
                        source: Arc::clone(origin),
                        offset,
                    })
                    .cloned()
                    .flatten()
            });
        }
        binding
    }
}

fn unobserved_command_boundary(
    state: &ModuleCommandBindings,
    native: SourceNativeInvocation<'_>,
) -> bool {
    let Some(identity) = &native.target.identity else {
        return false;
    };
    if state.has_opaque_domain()
        || state.source_step_observed()
        || state.source_execution_observed(Some(identity))
    {
        return false;
    }
    let Some(entry) = &state.baseline.native_entry else {
        return identity.runtime.is_none() && native.target.registry_backed;
    };
    identity.runtime.is_some_and(|runtime| {
        runtime.interpreter == entry.interpreter
            && entry
                .commands
                .iter()
                .any(|command| command.token == runtime.token && !command.has_execution_trace)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn captured_normal_store_continuation_requires_closed_command_boundary() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile)
            .commands()
            .clone();
        for (variable_trace, execution_trace, expected) in [
            ("trace add variable x write observe; ", "", true),
            (
                "trace add variable x write observe; ",
                "proc execution args {}; trace add execution set leave execution; ",
                false,
            ),
            ("", "", false),
        ] {
            let source = format!(
                "proc observe args {{}}; set x before; {variable_trace}{execution_trace}set x after; list READY"
            );
            let analysed = super::super::SourceCommandBindings::analyse_with_options(&source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar), &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                        ..Default::default()
                    }, ..Default::default()
                });
            let binding = analysed.invocation_at_source(
                "set",
                u32::try_from(source.rfind("set x after").unwrap()).unwrap(),
            );
            assert_eq!(
                binding.normal_variable_continuation().is_some(),
                expected,
                "{source}"
            );
            if let Some(context) = binding.normal_variable_continuation() {
                assert_eq!(context.literal_value("x", &registry), Some("after"));
            }
        }
    }
}
