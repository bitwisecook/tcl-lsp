// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rule-loader declarations retain deferred inventory independently of compilation.

use super::{
    ModuleCommandBindings, SourceCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceNativeInvocation, SourceOriginKind, prepare_source_native_invocation, source_binding,
};

/// A validated declaration alternative at its exact rule-load source site.
/// This inventory grants neither installation, execution nor compiler admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRuleDeclarationCandidate {
    /// Actual retained command implementation for this alternative.
    pub target: SourceCommandTarget,
    /// Registry-validated declaration and original argument positions.
    pub declaration: tcl_registry::events::IrulesTopLevelDeclaration,
    /// Exact retained deferred body; this does not assert that it was entered.
    pub body: Option<super::ExecutedScriptSource>,
    /// Unenumerated handler alternatives remain possible at this registration.
    pub unknown: bool,
}

impl SourceCommandBindings {
    /// Validated deferred declaration alternatives in the selected source instance.
    /// An absent inventory is uncertainty, rather than a spelling-based fallback.
    #[must_use]
    pub fn deferred_rule_declaration_candidates(
        &self,
        offset: u32,
    ) -> Option<&[SourceRuleDeclarationCandidate]> {
        self.rule_declarations
            .get(&super::CommandAllocationSite {
                source: super::Arc::clone(self.root_origin.as_ref()?),
                offset,
            })
            .map(Vec::as_slice)
    }

    fn record_rule_declaration(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        incoming: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        declaration: (
            tcl_registry::events::IrulesTopLevelDeclaration,
            Option<super::ExecutedScriptSource>,
        ),
        context: SourceExecutionContext<'_>,
    ) {
        let Some(source) = &incoming.current_source_origin else {
            return;
        };
        let body = declaration.1.or_else(|| {
            self.deferred.values().find_map(|body| {
                let (parent, _, script) = body.executed_script.as_ref()?;
                (parent.source == *source && parent.offset == segment.span.start())
                    .then(|| script.clone())
            })
        });
        let binding = source_binding(incoming, segment.name(), &context.namespace_identity());
        let candidate = SourceRuleDeclarationCandidate {
            target: target.clone(),
            declaration: declaration.0,
            body,
            unknown: binding.proved_target() != Some(target),
        };
        let candidates = self
            .rule_declarations
            .entry(super::CommandAllocationSite {
                source: super::Arc::clone(source),
                offset: segment.span.start(),
            })
            .or_default();
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    #[inline(never)]
    pub(super) fn retain_rule_loader_events(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        if context.realm != tcl_dialect::model::InvocationRealm::RuleLoader {
            return;
        }
        let Some(head) = effective
            .first()
            .and_then(|word| word.as_registry_word().literal())
        else {
            return;
        };
        // These are actual retained runtime alternatives. An opaque branch
        // remains opaque; the deferred inventory asserts neither installation
        // nor that this candidate is the unique handler.
        let binding = source_binding(incoming, head, &context.namespace_identity());
        for target in &binding.targets {
            self.retain_rule_loader_event(segment, words, effective, incoming, target, context);
            self.retain_rule_loader_file_policy(segment, effective, incoming, target, context);
        }
    }

    fn retain_rule_loader_file_policy(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) {
        if !rule_loader_target_is_eligible(target, context)
            || context.registry.irules_command_placement(
                &target.command,
                tcl_registry::events::IrulesExecutionContext::TopLevel,
            ) != tcl_registry::events::IrulesCommandPlacement::Allowed
        {
            return;
        }
        let Some(values) = effective
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let Some(declaration) = context
            .registry
            .irules_top_level_effect(&target.command, &values)
        else {
            return;
        };
        self.record_rule_declaration(segment, incoming, target, (declaration, None), context);
    }

    #[inline(never)]
    fn retain_rule_loader_event(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) {
        let Some(declaration) = validated_rule_declaration(segment, incoming, target, context)
        else {
            return;
        };
        let body_index = match &declaration {
            tcl_registry::events::IrulesTopLevelDeclaration::Event { body_index, .. }
            | tcl_registry::events::IrulesTopLevelDeclaration::Procedure { body_index, .. } => {
                *body_index
            }
            _ => return,
        };
        let Ok(prepared) = prepare_source_native_invocation(
            target,
            effective,
            incoming,
            context.registry,
            context.realm,
        ) else {
            return;
        };
        let mut invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&target.command),
            &prepared.arguments,
        );
        if let Some(dialect) = prepared.dialect {
            invocation = invocation.with_dialect(dialect);
        }
        let selection = tcl_registry::native_compilation::NativeCompilationSelection::Unknown;
        let native = SourceNativeInvocation {
            compilation_spec: prepared.facts.native_compilation.as_ref(),
            compilation_selection: &selection,
            segment,
            words,
            written_arguments: context.written_arguments,
            target,
            invocation: &invocation,
        };
        let body = super::retained_script_operand(
            body_index,
            native.script_operands(),
            incoming,
            segment.span.start(),
            context.config,
        );
        if matches!(
            declaration,
            tcl_registry::events::IrulesTopLevelDeclaration::Event { .. }
        ) {
            self.register_deferred_event(native, incoming, context, &prepared.facts);
        }
        self.record_rule_declaration(segment, incoming, target, (declaration, body), context);
    }

    /// The loader's validated procedure installation is independent of the
    /// unknown host compiler. Only its normal continuation publishes the name;
    /// an unresolved compiler failure still stops before later declarations.
    #[inline(never)]
    pub(super) fn walk_rule_loader_procedure(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<super::SourceOutcomes> {
        let declaration = validated_rule_declaration(segment, incoming, target, context)?;
        if !matches!(
            declaration,
            tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
        ) {
            return None;
        }
        let mut normal = super::boxed_source_branch(incoming);
        let mut outcomes = self.walk_source_target(
            segment,
            words,
            effective,
            &mut normal,
            target,
            &SourceExecutionContext {
                selected_compilation: Some(
                    &tcl_registry::native_compilation::NativeCompilationSelection::Unknown,
                ),
                ..context
            },
        );
        self.record_rule_declaration(segment, incoming, target, (declaration, None), context);
        outcomes.add_abrupt(
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Error,
            ),
            incoming,
        );
        Some(outcomes)
    }
}

fn rule_loader_target_is_eligible(
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> bool {
    context.realm == tcl_dialect::model::InvocationRealm::RuleLoader
        && context
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
        && matches!(
            context.frame,
            crate::var_resolve::VariableExecutionFrame::Global
        )
        && target.registry_backed
        && target.prepended.is_empty()
}

fn validated_rule_declaration(
    segment: &crate::segmenter::SegmentedCommand,
    incoming: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<tcl_registry::events::IrulesTopLevelDeclaration> {
    use tcl_registry::events::IrulesDeclarationArguments;
    if !rule_loader_target_is_eligible(target, context) {
        return None;
    }
    let SourceOriginKind::Authored(source) = incoming.current_source_origin.as_ref()?.kind() else {
        return None;
    };
    let source = source.try_text().ok()?;
    let closed = tcl_registry::events::closed_braced_argument_words(
        source,
        segment.arg_tokens(),
        segment.arg_single_token(),
    )?;
    let arguments = segment
        .args()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let arguments = IrulesDeclarationArguments::new(
        &arguments,
        segment.arg_tokens(),
        segment.arg_single_token(),
        &closed,
    )?;
    context
        .registry
        .irules_top_level_declaration_shape(&target.command, arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_registry::model::ingress::resolve_environment("f5-irules").unit_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn compiler_unknown_keeps_only_the_validated_rule_loader_event_inventory() {
        let source = "when HTTP_REQUEST {set selected YES}";
        let bindings = analyse(source);
        assert!(bindings.deferred.values().any(|body| {
            body.event.as_deref() == Some("HTTP_REQUEST")
                && body.source == "set selected YES"
                && body.source_origin.as_ref() == bindings.root_origin.as_ref()
        }));
        let invocation = bindings.invocation_at_source("when", 0);
        assert!(invocation.proved_execution_target().is_none());
        assert_eq!(
            invocation.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn rule_loader_inventory_declines_unbraced_and_replaced_event_handlers() {
        for source in [
            "when HTTP_REQUEST bare",
            "proc when args {return CUSTOM}; when HTTP_REQUEST {set selected YES}",
        ] {
            assert!(
                analyse(source)
                    .deferred
                    .values()
                    .all(|body| body.event.is_none()),
                "{source}"
            );
        }
    }

    #[test]
    fn frozen_priority_retains_only_the_rule_loader_file_policy_value() {
        let source = "set selected 700; priority $selected; when HTTP_REQUEST {set later YES}";
        let bindings = analyse(source);
        let offset = u32::try_from(source.find("priority").unwrap()).unwrap();
        let candidates = bindings
            .deferred_rule_declaration_candidates(offset)
            .unwrap();
        assert!(candidates.iter().any(|candidate| {
            matches!(
                candidate.declaration,
                tcl_registry::events::IrulesTopLevelDeclaration::Priority { value: 700 }
            ) && !candidate.unknown
                && candidate.body.is_none()
        }));
        assert!(
            bindings
                .invocation_at_source("", offset)
                .proved_execution_target()
                .is_none()
        );
    }

    #[test]
    fn earlier_compiler_residual_does_not_erase_later_possible_event_inventory() {
        let bindings =
            analyse("when CLIENT_ACCEPTED {set selected YES}; when HTTP_REQUEST {set later YES}");
        let events = bindings
            .deferred
            .values()
            .filter_map(|body| body.event.as_deref())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            events,
            std::collections::BTreeSet::from(["CLIENT_ACCEPTED", "HTTP_REQUEST"])
        );
        let offset = u32::try_from(
            "when CLIENT_ACCEPTED {set selected YES}; when HTTP_REQUEST {set later YES}"
                .find("when HTTP_REQUEST")
                .unwrap(),
        )
        .unwrap();
        let candidates = bindings
            .deferred_rule_declaration_candidates(offset)
            .unwrap();
        assert!(candidates.iter().all(|candidate| candidate.unknown));
        assert!(candidates.iter().any(|candidate| {
            candidate
                .body
                .as_ref()
                .is_some_and(|body| body.text.try_text().unwrap() == "set later YES")
        }));
    }

    #[test]
    fn direct_loader_keeps_later_procedure_body_without_installation_proof() {
        let source = "when HTTP_REQUEST {set first YES}; unresolvedOperation; proc helper {value} {return $value}";
        let profile = tcl_registry::model::ingress::resolve_environment("f5-irules").unit_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("proc helper").unwrap()).unwrap();
        let candidates = bindings
            .deferred_rule_declaration_candidates(offset)
            .unwrap();
        assert!(candidates.iter().any(|candidate| {
            matches!(
                candidate.declaration,
                tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
            ) && candidate
                .body
                .as_ref()
                .is_some_and(|body| body.text.try_text().unwrap() == "return $value")
        }));
        assert!(candidates.iter().all(|candidate| candidate.unknown));
        assert!(
            bindings
                .invocation_at_source("proc", offset)
                .proved_execution_target()
                .is_none()
        );
    }

    #[test]
    fn direct_loader_registration_preserves_the_later_original_procedure_handler() {
        let source = "when HTTP_REQUEST {set first YES}; proc helper {value} {return $value}";
        let profile = tcl_registry::model::ingress::resolve_environment("f5-irules").unit_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("proc helper").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("proc", offset)
                .proved_execution_target()
                .is_some()
        );
        assert!(
            bindings
                .deferred_rule_declaration_candidates(offset)
                .unwrap()
                .iter()
                .all(|candidate| !candidate.unknown)
        );
    }
}
