// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original iRules source candidates, independent of execution or attachment.

fn selected_rule_procedure_operand(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<usize> {
    schema.authored_source_rule_procedure_operand()
}

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ir::{Module, SourceIrulesEventBody};
use tcl_compiler::registry_invocation::source_structure::{
    OriginalRegistryWords, source_registry_words,
};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::model::ContextRegistry;
use tcl_registry::{CommandRegistry, RegistrySemanticKey, Traits};
use tcl_syntax::naming::VendorSourceNamePurpose;

/// Unmet report premise. These source candidates cannot certify execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesSourceObligation {
    /// The original source schema has independent applicability obligations.
    ConditionalApplicability,
    /// A called procedure's original source target is unavailable or external.
    ProcedureTargetUnavailable,
    /// Matching source declarations remain alternatives without runtime binding.
    ProcedureBindingUnavailable,
}

/// One guarded authored source vector and optional real event-body provenance.
/// Its canonical schema label is presentation, not a selected live handler.
#[derive(Debug, Clone)]
pub struct OriginalIrulesSourceCommand {
    span: Span,
    words: OriginalRegistryWords,
    event: SourceIrulesEventBody,
    obligations: Vec<IrulesSourceObligation>,
}

impl OriginalIrulesSourceCommand {
    /// Actual complete command extent in the independently retained document.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Shared Compiler source argv, original origins and applicability carrier.
    #[must_use]
    pub const fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }

    /// Real lowering descriptor, without an entered event, worker or epoch.
    #[must_use]
    pub const fn event_source(&self) -> &SourceIrulesEventBody {
        &self.event
    }

    /// Unmet premises remain explicit rather than becoming behavior assertions.
    #[must_use]
    pub fn obligations(&self) -> &[IrulesSourceObligation] {
        &self.obligations
    }
}

/// A sealed current document's conditional event-rooted source closure.
/// Source procedure choices preserve every matching declaration site. They
/// provide no reached call, native command identity, worker or edit authority.
#[derive(Debug, Clone)]
pub struct OriginalIrulesSourceContext {
    image: SourceImage,
    config: LexerConfig,
    context: tcl_registry::model::ResolvedContext,
    context_registry: Arc<ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    registry: RegistrySemanticKey,
    events: Vec<SourceIrulesEventBody>,
    commands: Vec<OriginalIrulesSourceCommand>,
    source_vectors: Vec<(Span, OriginalRegistryWords)>,
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

// Labels preserve authentic variable-token geometry without issuing a cell key.
fn source_variable_label(raw: &str) -> String {
    let raw = raw.strip_prefix('$').unwrap_or(raw);
    raw.strip_prefix('{')
        .and_then(|name| name.strip_suffix('}'))
        .unwrap_or(raw)
        .to_owned()
}

fn source_vectors_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
    context: &ContextRegistry,
) -> Option<Vec<(Span, OriginalRegistryWords)>> {
    let mut vectors = Vec::new();
    for occurrence in analysis.original_vendor_source_names() {
        let original = occurrence.original_words();
        let (Some(first), Some(last)) = (original.first(), original.last()) else {
            continue;
        };
        if first != occurrence.name_input().original_word() {
            continue;
        }
        let start = first.span().start();
        let end = last.span().end();
        let text = source.get(start as usize..end as usize)?;
        let mut segments =
            tcl_compiler::segmenter::segment_commands_with_offset_and_config(text, start, config);
        if segments.len() != 1 {
            continue;
        }
        let segment = segments.remove(0);
        let Some(words) = source_registry_words(source, analysis, &segment) else {
            continue;
        };
        if words.matches_source(image, config)
            && words.matches_registry(context.commands())
            && words.context() == Some(context.context())
        {
            vectors.push((segment.span, words));
        }
    }
    vectors.sort_by_key(|(span, _)| span.start());

    Some(vectors)
}

type OriginalSourceBodyQueue = VecDeque<(Span, usize, bool)>;

fn excluded_source_bodies(
    vectors: &[(Span, OriginalRegistryWords)],
    context: &ContextRegistry,
) -> Vec<Span> {
    use tcl_compiler::registry_invocation::OriginalSourceScriptPurpose;
    vectors
        .iter()
        .flat_map(|(_, words)| {
            let deferred = words.with_source_schema(context, |schema| {
                schema.semantics.traits.contains(Traits::DEFERS_BODY)
            }) == Some(true);
            let potential = words.source_script_bodies_for(
                context,
                OriginalSourceScriptPurpose::PotentialEvaluation,
            );
            words
                .source_script_bodies(context)
                .into_iter()
                .filter(move |body| {
                    deferred
                        || !potential
                            .iter()
                            .any(|other| other.content_span() == body.content_span())
                })
                .map(|body| body.content_span())
        })
        .collect()
}

fn source_event_roots(
    source: &str,
    module: &Module,
    config: LexerConfig,
    vectors: &[(Span, OriginalRegistryWords)],
    context: &ContextRegistry,
) -> (Vec<SourceIrulesEventBody>, OriginalSourceBodyQueue) {
    let top_level =
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .map(|segment| segment.span.start())
            .collect::<HashSet<_>>();
    let mut events = Vec::new();
    let mut pending = VecDeque::new();
    for (label, procedure) in &module.procedures {
        let Some(event) = tcl_compiler::source_graph::event_body_for_procedure(
            module,
            label,
            procedure,
            context.commands(),
        ) else {
            continue;
        };
        if !top_level.contains(&procedure.span.start()) {
            continue;
        }
        let Some((_, words)) = vectors
            .iter()
            .find(|(span, _)| span.start() == procedure.span.start())
        else {
            continue;
        };
        if words.with_source_schema(context, |schema| {
            schema.semantics.traits.contains(Traits::IS_EVENT_HANDLER)
        }) != Some(true)
        {
            continue;
        }
        let bodies = words.source_script_bodies_for(
            context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        );
        if bodies.len() != 1 {
            continue;
        }
        let index = events.len();
        events.push(event.clone());
        pending.push_back((bodies[0].content_span(), index, false));
    }
    (events, pending)
}

/// Genuine declaration and shared source-body issuer supply the same complete
/// source call. Reported token endpoints cannot replace body content geometry.
fn source_procedure_declarations(
    analysis: &AnalysisResult,
    vectors: &[(Span, OriginalRegistryWords)],
    image: &SourceImage,
    config: LexerConfig,
    context: &ContextRegistry,
) -> Option<Vec<(Vec<u8>, Span)>> {
    // naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    use tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource;
    let mut declarations = Vec::new();
    for declaration in analysis.original_vendor_procedure_declarations() {
        if !declaration.name_input().matches_source(image, config) {
            return None;
        }
        let Some(name) = declaration
            .name_input()
            .literal_units(VendorSourceNamePurpose::ProcedureName)
        else {
            continue;
        };
        let occurrence = declaration.original_occurrence();
        let Some((_, words)) = vectors.iter().find(|(span, words)| {
            span.start() == occurrence.site().offset
                && matches!(words.source(), OriginalRegistrySource::Vendor(metadata)
                    if metadata.shape().original_words() == occurrence.original_words())
        }) else {
            continue;
        };
        let bodies = words.source_script_bodies(context);
        let [body] = bodies.as_slice() else {
            continue;
        };
        if body.matches_source(image, config)
            && occurrence
                .original_words()
                .contains(body.original_container())
        {
            declarations.push((name.to_vec(), body.content_span()));
        }
    }
    Some(declarations)
}

fn collect_source_commands(
    mut pending: OriginalSourceBodyQueue,
    events: &[SourceIrulesEventBody],
    vectors: &[(Span, OriginalRegistryWords)],
    excluded_bodies: &[Span],
    declarations: &[(Vec<u8>, Span)],
    context: &ContextRegistry,
) -> Vec<OriginalIrulesSourceCommand> {
    let mut visited = HashSet::new();
    let mut commands = Vec::new();
    while let Some((body, event, through_call)) = pending.pop_front() {
        if !visited.insert((body.start(), body.end(), event)) {
            continue;
        }
        for (span, words) in vectors.iter().filter(|(span, _)| contains(body, *span)) {
            if excluded_bodies
                .iter()
                .any(|excluded| *excluded != body && contains(*excluded, *span))
            {
                continue;
            }
            let mut obligations = vec![IrulesSourceObligation::ConditionalApplicability];
            if through_call {
                obligations.push(IrulesSourceObligation::ProcedureBindingUnavailable);
            }
            if let Some(Some(argument)) =
                words.with_source_schema(context, selected_rule_procedure_operand)
            {
                let target = words
                    .arguments()
                    .get(argument)
                    .and_then(|word| word.literal_bytes());
                let candidates = declarations
                    .iter()
                    .filter(|(name, _)| Some(name.as_slice()) == target)
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    obligations.push(IrulesSourceObligation::ProcedureTargetUnavailable);
                } else {
                    obligations.push(IrulesSourceObligation::ProcedureBindingUnavailable);
                    for (_, span) in candidates {
                        pending.push_back((*span, event, true));
                    }
                }
            }
            commands.push(OriginalIrulesSourceCommand {
                span: *span,
                words: words.clone(),
                event: events[event].clone(),
                obligations,
            });
        }
    }
    commands.sort_by_key(|command| command.span.start());
    commands
}

impl OriginalIrulesSourceContext {
    /// Retain the actual source analysis, lowered event descriptors and complete
    /// availability context. No grammar or context is rebuilt from labels.
    #[must_use]
    pub fn from_analysis(
        source: &str,
        analysis: &AnalysisResult,
        module: &Module,
        context: &ContextRegistry,
    ) -> Option<Self> {
        // Implementation contract: naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        let (image, config) = tcl_compiler::source_graph::current_analysis(source, analysis)?;
        let retained = analysis.resolved_input.as_ref()?;
        if !retained.unit_profile().is_irules()
            || !analysis.has_original_vendor_source_names()
            || retained.context_registry().context() != context.context()
            || retained
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != context.commands().snapshot().semantic_key()
            || module.source != image
            || module.lexer_config != config
            || module.registry_snapshot.as_ref()?.semantic_key()
                != context.commands().snapshot().semantic_key()
        {
            return None;
        }

        let vectors = source_vectors_from_analysis(source, analysis, &image, config, context)?;

        let excluded_bodies = excluded_source_bodies(&vectors, context);

        let (events, pending) = source_event_roots(source, module, config, &vectors, context);
        let declarations =
            source_procedure_declarations(analysis, &vectors, &image, config, context)?;
        let commands = collect_source_commands(
            pending,
            &events,
            &vectors,
            &excluded_bodies,
            &declarations,
            context,
        );
        Some(Self {
            image,
            config,
            context: context.context().clone(),
            registry: context.commands().snapshot().semantic_key(),
            context_registry: retained.context_registry(),
            profile: retained.analyser_profile(),
            events,
            commands,
            source_vectors: vectors,
        })
    }

    /// Retain only readonly source vectors from the actual current analysis.
    /// Full image/configuration, availability context and Registry remain owned
    /// by that analysis. No module is rebuilt and no event/body entry is issued.
    #[must_use]
    pub fn from_source_analysis(source: &str, analysis: &AnalysisResult) -> Option<Self> {
        // Implementation contract: naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        let (image, config) = tcl_compiler::source_graph::current_analysis(source, analysis)?;
        let retained = analysis.resolved_input.as_ref()?;
        let context = retained.context_registry();
        let registry = analysis.resolved_registry()?;
        if !retained.unit_profile().is_irules()
            || !analysis.has_original_vendor_source_names()
            || context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key()
        {
            return None;
        }
        let vectors = source_vectors_from_analysis(source, analysis, &image, config, &context)?;
        Some(Self {
            image,
            config,
            context: context.context().clone(),
            registry: registry.snapshot().semantic_key(),
            context_registry: context,
            profile: retained.analyser_profile(),
            events: Vec::new(),
            commands: Vec::new(),
            source_vectors: vectors,
        })
    }

    /// Explicit source-only iRules API ingress under the supplied actual store.
    /// The selected complete profile/configuration are captured at analysis;
    /// callers retaining independent overrides use `from_analysis` instead.
    #[must_use]
    pub fn capture(source: &str, registry: &CommandRegistry) -> Option<Self> {
        crate::executable::record_source_capture();
        let profile = registry
            .profile()
            .filter(|profile| profile.is_irules())
            .unwrap_or_else(tcl_dialect::DialectProfile::irules);
        let context = Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let config = LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let unit = Arc::new(CompilationUnit::build_with_context_registry(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            context.clone(),
        ));
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(unit.clone());
        let analysis = analyser.analyse(source, profile.name);
        Self::from_analysis(source, &analysis, &unit.ir_module, &context)
    }

    /// Complete current image/configuration and actual context/store check.
    #[must_use]
    pub fn matches_current(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        context: &ContextRegistry,
    ) -> bool {
        tcl_compiler::source_graph::current_analysis(source, analysis)
            .is_some_and(|(image, config)| self.image == image && self.config == config)
            && self.context == *context.context()
            && self.registry == context.commands().snapshot().semantic_key()
            && analysis.resolved_input.as_ref().is_some_and(|input| {
                input.context_registry().context() == context.context()
                    && input
                        .context_registry()
                        .commands()
                        .snapshot()
                        .semantic_key()
                        == self.registry
            })
    }

    /// Compare the complete original document image before projecting source cards.
    #[must_use]
    pub fn matches_source(&self, source: &str) -> bool {
        self.image == SourceImage::document(source)
    }

    /// Exact immutable source-schema store/context, without a runtime table.
    #[must_use]
    pub fn context_registry(&self) -> &ContextRegistry {
        &self.context_registry
    }

    /// Presentation-only compatibility fields from genuine source candidates.
    /// Unknown operands display their own written source rather than pretending
    /// to be literal values. These strings cannot select identity or effects.
    #[must_use]
    pub fn presentation_commands(&self, source: &str) -> Vec<crate::IrulesExecutableCommand> {
        if self.image != SourceImage::document(source) {
            return Vec::new();
        }
        self.commands
            .iter()
            .map(|command| {
                let args = command
                    .words
                    .arguments()
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        value
                            .literal_bytes()
                            .and_then(|bytes| std::str::from_utf8(bytes).ok())
                            .map(str::to_owned)
                            .or_else(|| {
                                let span = command.words.operands().get(index)?.as_ref()?.span();
                                source
                                    .get(span.start() as usize..span.end() as usize)
                                    .map(str::to_owned)
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                let mut variable_names = Vec::new();
                for operand in command.words.operands().iter().flatten() {
                    if let Some(word) = operand.word() {
                        for token in word
                            .tokens()
                            .iter()
                            .filter(|token| token.kind == tcl_lexer::TokenType::Var)
                        {
                            let start = token
                                .span
                                .start()
                                .saturating_add(u32::from(token.content_offset));
                            if let Some(raw) = source.get(start as usize..token.span.end() as usize)
                            {
                                variable_names.push(source_variable_label(raw));
                            }
                        }
                    }
                }
                for &(index, role) in command.words.roles().unwrap_or_default() {
                    if role != tcl_registry::ArgRole::Expr {
                        continue;
                    }
                    let Some(word) = command
                        .words
                        .operands()
                        .get(index)
                        .and_then(Option::as_ref)
                        .and_then(|operand| operand.word())
                    else {
                        continue;
                    };
                    let Some(token) = word.tokens().first() else {
                        continue;
                    };
                    if source.as_bytes().get(token.span.start() as usize) != Some(&b'{')
                        || token.content_offset != 1
                    {
                        continue;
                    }
                    let Ok(span) = word.content_span() else {
                        continue;
                    };
                    let Some(expression) = source.get(span.as_range()) else {
                        continue;
                    };
                    let surface =
                        tcl_registry::expr_surface::RuntimeExprSurface::for_profile(self.profile);
                    let substitutions = tcl_syntax::expr::live_expression_substitutions(
                        expression,
                        self.profile,
                        self.config,
                        |parsed| surface.validate(parsed).is_ok(),
                    );
                    for variable in substitutions.variables {
                        let start = span.start().saturating_add(variable.start());
                        let end = span.start().saturating_add(variable.end());
                        if let Some(raw) = source.get(start as usize..end as usize) {
                            variable_names.push(source_variable_label(raw));
                        }
                    }
                }
                crate::IrulesExecutableCommand {
                    span: command.span,
                    command: command.words.command().to_owned(),
                    args,
                    variable_names,
                    event: Some(command.event.event().to_owned()),
                }
            })
            .collect()
    }

    /// Complete guarded authored source vectors, including stored bodies.
    /// Membership is source syntax, independent of event reachability or calls.
    #[must_use]
    pub fn source_vectors(&self) -> &[(Span, OriginalRegistryWords)] {
        &self.source_vectors
    }

    /// Actual event-body declarations; presence is not event reachability.
    #[must_use]
    pub fn events(&self) -> &[SourceIrulesEventBody] {
        &self.events
    }

    /// Conditional potential-evaluation paths and matching procedure declarations.
    /// Reference-only bodies remain in the independent source-vector inventory.
    #[must_use]
    pub fn commands(&self) -> &[OriginalIrulesSourceCommand] {
        &self.commands
    }

    /// Exact availability context retained by the original analysis ingress.
    #[must_use]
    pub const fn context(&self) -> &tcl_registry::model::ResolvedContext {
        &self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_irules_source_context_keeps_event_receipts_and_handler_barriers() {
        // Implementation contract: naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "proc helper {} {pool from_helper}\nproc dormant {} {pool dormant}\nwhen HTTP_REQUEST {call helper; pool direct; snatpool source_snat}";
        let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        assert_eq!(inventory.events().len(), 1);
        assert!(inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"from_helper")
        }));
        assert!(!inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"dormant")
        }));
        assert!(
            inventory
                .commands()
                .iter()
                .all(|command| !command.obligations().is_empty())
        );
        assert_eq!(inventory.events()[0].event(), "HTTP_REQUEST");
        for source in [
            "proc when args {}; when HTTP_REQUEST {pool bad}",
            "proc pool args {}; when HTTP_REQUEST {pool bad}",
            "proc ::pool args {}; when HTTP_REQUEST {::pool bad}",
        ] {
            let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
            assert!(
                !inventory
                    .commands()
                    .iter()
                    .any(|command| command.words().command() == "pool"),
                "{source}: a genuine source replacement blocks catalogue advice"
            );
        }
    }
    #[test]
    fn original_irules_unavailable_rename_retains_only_conditional_source_advice() {
        // naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        // BIG-IP's separate literal-head loader observation rejects rename:
        // docs/design/analysis/name-resolution-proofs/bigip-literal-head-loader-boundary.md
        // No selected TMM move descriptor means no known source tombstone.
        // The remaining source path grants no accepted rule or event execution.
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "rename pool moved; when HTTP_REQUEST {pool conditional}";
        let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let pool = inventory
            .commands()
            .iter()
            .find(|command| command.words().command() == "pool")
            .expect("conditional source schema remains separate from loader admission");
        let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
            metadata,
        ) = pool.words().source()
        else {
            panic!("hosted advice must retain its authentic source issuer");
        };
        assert!(metadata.authored_barriers().has_unknown_transitions());
        assert!(
            pool.obligations()
                .contains(&IrulesSourceObligation::ConditionalApplicability)
        );
    }

    fn reference_only_script(
        _arguments: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }

    #[test]
    fn original_irules_source_paths_keep_reference_only_bodies_as_syntax() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        // Same whole source and Registry role; only selected timing changes.
        // Neither branch supplies event execution or native body entry.
        let profile = tcl_dialect::DialectProfile::irules();
        let source = "when HTTP_REQUEST {source-body {pool nested}; pool direct}";
        for reference_only in [true, false] {
            let mut registry = CommandRegistry::build_default().project_for_profile(profile);
            registry.insert(tcl_registry::CommandSpec {
                name: "source-body",
                arity: tcl_registry::Arity::exact(1),
                arg_roles: &[(0, tcl_registry::ArgRole::Body)],
                script_timing_resolver: reference_only.then_some(reference_only_script),
                ..tcl_registry::CommandSpec::DEFAULT
            });
            let inventory = OriginalIrulesSourceContext::capture(source, &registry).unwrap();
            let is_pool = |words: &OriginalRegistryWords, name: &[u8]| {
                words.command() == "pool"
                    && words
                        .arguments()
                        .first()
                        .and_then(|word| word.literal_bytes())
                        == Some(name)
            };
            assert!(
                inventory
                    .source_vectors()
                    .iter()
                    .any(|(_, words)| is_pool(words, b"nested")),
                "readonly original syntax remains independently visible"
            );
            assert!(
                inventory
                    .commands()
                    .iter()
                    .any(|command| is_pool(command.words(), b"direct"))
            );
            assert_eq!(
                inventory
                    .commands()
                    .iter()
                    .any(|command| is_pool(command.words(), b"nested")),
                !reference_only,
                "{reference_only}: only potential evaluation supplies an event-rooted path"
            );
        }
    }

    #[test]
    fn original_irules_called_body_uses_complete_source_geometry_and_not_proc_reports() {
        // naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = tcl_dialect::DialectProfile::irules();
        let context = Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let config = LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let source = "proc helper {} {pool first; pool final}\nproc dormant {} {pool dormant}\nwhen HTTP_REQUEST {call helper}";
        let unit = Arc::new(CompilationUnit::build_with_context_registry(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            context.clone(),
        ));
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(unit.clone());
        let mut analysis = analyser.analyse(source, profile.name);
        analysis.all_procs.clear();
        let inventory = OriginalIrulesSourceContext::from_analysis(
            source,
            &analysis,
            &unit.ir_module,
            &context,
        )
        .unwrap();
        for operand in [b"first".as_slice(), b"final".as_slice()] {
            let command = inventory
                .commands()
                .iter()
                .find(|command| {
                    command
                        .words()
                        .arguments()
                        .first()
                        .and_then(|word| word.literal_bytes())
                        == Some(operand)
                })
                .expect("complete genuine called source body");
            assert!(
                command
                    .obligations()
                    .contains(&IrulesSourceObligation::ProcedureBindingUnavailable)
            );
            assert_eq!(command.event_source().event(), "HTTP_REQUEST");
        }
        assert!(!inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"dormant")
        }));
    }
}
