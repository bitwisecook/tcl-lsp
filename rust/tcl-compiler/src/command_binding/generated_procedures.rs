// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original procedure body coverage, independently of callable IR registration.

use super::{
    AllocationIncarnation, BindingKind, CommandAllocation, DeferredImplementationId,
    ExecutedScriptSource, MayBinding, SourceCommandBindings, SourceCommandTarget,
};

/// Original body of one exact retained procedure implementation. The caller
/// independently proves that implementation is selected at its use site.
/// This inventory supplies no body-entry, completion or compiler authority.
#[derive(Debug, Clone, Copy)]
pub struct SourceProcedureImplementationBody<'a> {
    /// Full implementation allocation, including its incarnation.
    pub allocation: &'a CommandAllocation,
    /// Original namespace presentation, with no native lookup authority.
    pub namespace: &'a str,
    /// Exact original namespace allocation or native incarnation.
    pub namespace_key: &'a super::SourceNamespaceKey,
    /// Original native formal binding plan.
    pub parameters: &'a [tcl_syntax::formal_params::FormalParameter],
    /// Original evaluated body source and provenance.
    pub source: &'a ExecutedScriptSource,
}

#[cfg(test)]
mod implementation_tests {
    use super::*;

    #[test]
    fn historical_procedure_bodies_require_the_full_selected_implementation() {
        let source = "proc f {} {return FIRST}; f; proc f {} {return SECOND}; f";
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = owner.commands().profile().expect("actual Tcl 8.6 fixture");
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            owner.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let bodies = source
            .match_indices("; f")
            .zip(["return FIRST", "return SECOND"])
            .map(|((offset, _), expected)| {
                let binding =
                    bindings.invocation_at_source("f", u32::try_from(offset + 2).unwrap());
                let target = binding
                    .proved_handler_target()
                    .expect("actual procedure target");
                let body = bindings
                    .procedure_implementation_body(target)
                    .expect("original body");
                assert_eq!(body.source.text.try_text().unwrap(), expected);
                assert_eq!(
                    Some(body.allocation),
                    target.implementation_allocation.as_ref()
                );
                body.allocation.clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 2);
        assert_ne!(bodies[0], bodies[1]);
        let inventory = bindings
            .procedure_implementation_bodies()
            .map(|body| body.allocation.clone())
            .collect::<Vec<_>>();
        assert!(
            bodies
                .iter()
                .all(|allocation| inventory.contains(allocation))
        );
    }
}

/// Original body metadata retained for analysis coverage. The issuing query
/// independently selects original declarations or currently installed bodies.
/// This supplies no callable, body-entry or compilation authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInstalledProcedureBody {
    /// Exact native definition allocation, including its source instance.
    pub allocation: CommandAllocation,
    /// Command presentation; native slot identity is retained independently.
    pub command: String,
    /// Namespace presentation selected by the issuing original-body query.
    pub namespace: String,
    /// Exact namespace context, independent of its display spelling.
    pub namespace_key: super::SourceNamespaceKey,
    /// Formals decoded by the selected native definition grammar.
    pub parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    /// Exact evaluated body and its retained source origin.
    pub source: ExecutedScriptSource,
}

impl SourceCommandBindings {
    /// Original procedure implementations retained for body analysis. This
    /// includes replaced declarations and does not select a callable at any site.
    /// Consumers match a separately proved current implementation allocation.
    pub fn procedure_implementation_bodies(
        &self,
    ) -> impl Iterator<Item = SourceProcedureImplementationBody<'_>> {
        self.deferred.values().filter_map(|root| {
            if root.receiver_method || root.event.is_some() || root.statics.is_some() {
                return None;
            }
            let allocation = root.implementation_allocation.as_ref()?;
            if allocation.incarnation == AllocationIncarnation::RepeatedFresh {
                return None;
            }
            let (_, _, source) = root.executed_script.as_ref()?;
            Some(SourceProcedureImplementationBody {
                allocation,
                namespace: &root.namespace,
                namespace_key: &root.namespace_key,
                parameters: &root.parameters,
                source,
            })
        })
    }

    /// Original inventory for this exact implementation, including replaced
    /// declarations. Current dispatch must come from a separate positioned receipt;
    /// final command names and future declarations cannot select this body.
    #[must_use]
    pub fn procedure_implementation_body(
        &self,
        target: &SourceCommandTarget,
    ) -> Option<SourceProcedureImplementationBody<'_>> {
        if target.kind != BindingKind::Proc {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        if allocation.incarnation == AllocationIncarnation::RepeatedFresh {
            return None;
        }
        let root = self.deferred.get(&target.implementation_id())?;
        if root.receiver_method || root.event.is_some() || root.statics.is_some() {
            return None;
        }
        let allocation = root
            .implementation_allocation
            .as_ref()
            .filter(|retained| *retained == allocation)?;
        let (_, _, source) = root.executed_script.as_ref()?;
        Some(SourceProcedureImplementationBody {
            allocation,
            namespace: &root.namespace,
            namespace_key: &root.namespace_key,
            parameters: &root.parameters,
            source,
        })
    }

    /// Retain every original declaration body with its exact site, source and
    /// namespace key. Final callable state is independent: deletion, replacement
    /// and an opaque continuation cannot erase original declaration coverage.
    /// This inventory never proves that a body entered or remains callable.
    #[must_use]
    pub fn original_declaration_body_units(&self) -> Vec<SourceInstalledProcedureBody> {
        let mut seen = std::collections::HashSet::new();
        let mut bodies = self
            .procedure_implementation_bodies()
            .filter(|body| {
                seen.insert((
                    body.allocation.clone(),
                    body.namespace_key.clone(),
                    body.source.clone(),
                ))
            })
            .map(|body| SourceInstalledProcedureBody {
                allocation: body.allocation.clone(),
                command: body.allocation.command.clone(),
                namespace: body.namespace.to_owned(),
                namespace_key: body.namespace_key.clone(),
                parameters: body.parameters.to_vec(),
                source: body.source.clone(),
            })
            .collect::<Vec<_>>();
        bodies.sort_by(|left, right| {
            left.allocation
                .cmp(&right.allocation)
                .then_with(|| left.namespace_key.cmp(&right.namespace_key))
        });
        bodies
    }

    /// Recover only singleton procedure implementations still installed at the
    /// document boundary. Retired/replaced, opaque, repeated and static-bound
    /// implementations decline; future-only declarations grant no evidence.
    #[must_use]
    pub fn installed_procedure_body_units(&self) -> Vec<SourceInstalledProcedureBody> {
        let state = &self.final_state;
        if state.has_opaque_domain() {
            return Vec::new();
        }
        let mut bodies = Vec::new();
        for (slot, bindings) in state.bindings.iter() {
            if bindings.len() != 1 {
                continue;
            }
            let Some(MayBinding::Target(target)) = bindings.first() else {
                continue;
            };
            if !target.terminal || target.kind != BindingKind::Proc || !target.prepended.is_empty()
            {
                continue;
            }
            let Some(allocation) = target
                .implementation_allocation
                .as_ref()
                .filter(|allocation| {
                    allocation.incarnation != AllocationIncarnation::RepeatedFresh
                })
            else {
                continue;
            };
            let Some(identity) = target.token.as_ref() else {
                continue;
            };
            let key = DeferredImplementationId {
                command: identity.origin.clone(),
                generation: target.implementation_generation,
                allocation: Some(allocation.clone()),
            };
            let Some(root) = self.deferred.get(&key).filter(|root| {
                root.event.is_none() && !root.receiver_method && root.statics.is_none()
            }) else {
                continue;
            };
            let Some(command) =
                crate::command_binding::ModuleCommandBindings::command_key_label(slot)
            else {
                continue;
            };
            let namespace_key = slot.holder().into_owned();
            let namespace = namespace_key.display().unwrap_or_default();
            let Some((_, _, source)) = &root.executed_script else {
                continue;
            };
            bodies.push(SourceInstalledProcedureBody {
                allocation: allocation.clone(),
                command,
                namespace,
                namespace_key,
                parameters: root.parameters.clone(),
                source: source.clone(),
            });
        }
        bodies.sort_by(|left, right| left.allocation.cmp(&right.allocation));
        bodies.dedup_by(|left, right| left.allocation == right.allocation);
        bodies
    }
}
