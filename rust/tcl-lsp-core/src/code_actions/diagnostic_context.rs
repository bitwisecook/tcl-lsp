// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained diagnostic subjects and current source-only authoring advice.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::Diagnostic;
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};
use tcl_registry::CommandRegistry;

/// A compiler-issued context diagnostic. Its rendered message carries no
/// identity. Wire values only reselect an existing bounded retained subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextDiagnosticData {
    diagnostic: Option<Arc<Diagnostic>>,
    notice: Option<super::spec_notice::SpecPackNoticeData>,
}

#[derive(Default)]
struct RetainedSubjects {
    next: u64,
    bytes: usize,
    entries: BTreeMap<u64, ContextDiagnosticData>,
    order: VecDeque<u64>,
}

fn subjects() -> &'static Mutex<RetainedSubjects> {
    static SUBJECTS: OnceLock<Mutex<RetainedSubjects>> = OnceLock::new();
    SUBJECTS.get_or_init(|| Mutex::new(RetainedSubjects::default()))
}

impl ContextDiagnosticData {
    /// Retain the actual compiler diagnostic without parsing its message.
    #[must_use]
    pub fn from_compiler_diagnostic(diagnostic: &Diagnostic) -> Option<Self> {
        let context = diagnostic.source_context.as_ref()?;
        if diagnostic.span.start() > diagnostic.span.end()
            || diagnostic.span.end() as usize > context.image().len()
        {
            return None;
        }
        Some(Self {
            diagnostic: Some(Arc::new(diagnostic.clone())),
            notice: None,
        })
    }

    /// Retain the actual loader-issued typed source notice, without its prose.
    #[must_use]
    pub fn from_spec_pack_notice(
        subject: &super::SpecPackNoticeSubject,
        analysis: &AnalysisResult,
    ) -> Option<Self> {
        Some(Self {
            diagnostic: None,
            notice: Some(super::spec_notice::SpecPackNoticeData::new(
                subject, analysis,
            )?),
        })
    }
    pub(super) fn notice(&self) -> Option<&super::spec_notice::SpecPackNoticeData> {
        self.notice.as_ref()
    }

    /// Process-local source-advice handle. Missing or evicted handles abstain.
    #[must_use]
    pub fn to_value(&self) -> serde_json::Value {
        if let Some(notice) = &self.notice {
            return notice.to_value();
        }
        let Some(diagnostic) = &self.diagnostic else {
            return serde_json::Value::Null;
        };
        const MAX_SUBJECTS: usize = 512;
        const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
        let mut retained = subjects()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let size = diagnostic
            .source_context
            .as_ref()
            .map_or(0, |context| context.image().len());
        if size > MAX_SOURCE_BYTES {
            return serde_json::Value::Null;
        }
        while retained.entries.len() >= MAX_SUBJECTS
            || retained.bytes.saturating_add(size) > MAX_SOURCE_BYTES
        {
            let Some(old) = retained.order.pop_front() else {
                return serde_json::Value::Null;
            };
            if let Some(old) = retained.entries.remove(&old) {
                retained.bytes = retained.bytes.saturating_sub(
                    old.diagnostic
                        .as_ref()
                        .and_then(|diagnostic| diagnostic.source_context.as_ref())
                        .map_or(0, |context| context.image().len()),
                );
            }
        }
        let Some(token) = retained.next.checked_add(1) else {
            return serde_json::Value::Null;
        };
        retained.next = token;
        retained.bytes += size;
        retained.entries.insert(token, self.clone());
        retained.order.push_back(token);
        serde_json::json!({ "kind":"tcl-source-diagnostic", "version":1, "token":token.to_string() })
    }

    /// Select an existing issuer subject; arbitrary JSON cannot construct one.
    #[must_use]
    pub fn from_value(value: &serde_json::Value, code: &str) -> Option<Self> {
        if value.get("kind")?.as_str()? == "spectcl-source-notice" {
            return Some(Self {
                diagnostic: None,
                notice: Some(super::spec_notice::SpecPackNoticeData::from_value(
                    value, code,
                )?),
            });
        }
        if value.get("kind")?.as_str()? != "tcl-source-diagnostic"
            || value.get("version")?.as_u64()? != 1
        {
            return None;
        }
        let token = value.get("token")?.as_str()?.parse::<u64>().ok()?;
        let retained = subjects()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let subject = retained.entries.get(&token)?;
        (subject.diagnostic.as_ref()?.code.as_str() == code).then(|| subject.clone())
    }

    pub(super) fn diagnostic(&self) -> Option<&Diagnostic> {
        self.diagnostic.as_deref()
    }

    pub(super) fn matches(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        registry: &CommandRegistry,
        diagnostic: &super::ContextDiagnostic,
    ) -> bool {
        let Some(diagnostic_subject) = self.diagnostic.as_ref() else {
            return false;
        };
        let Some(context) = diagnostic_subject.source_context.as_ref() else {
            return false;
        };
        let Some(config) = analysis.body_lexer_config else {
            return false;
        };
        let Some(current_registry) = analysis.resolved_registry() else {
            return false;
        };
        diagnostic_subject.code.as_str() == diagnostic.code
            && context.image() == &SourceImage::document(source)
            && context.lexer_config() == config
            && context.registry().semantic_key() == registry.snapshot().semantic_key()
            && context.registry().semantic_key() == current_registry.snapshot().semantic_key()
            && analysis.matches_original_source_image(context.image(), config)
            && crate::definition::span_to_range(
                source,
                &tcl_lexer::LineIndex::new(source),
                diagnostic_subject.span,
            ) == diagnostic.range
    }
}

/// One current selected source command. These are authoring/metadata facts,
/// not a live command, compiler preparation, Normal or runtime effect grant.
pub(super) struct SourceCommand {
    pub canonical: String,
    pub words: Vec<NativeWord>,
    pub span: Span,
}

/// Exact supported ASCII source literal, for authored Registry vocabulary.
/// Escaped/computed/expanded units remain unavailable rather than becoming a
/// native name or borrowing a reporting spelling.
pub(super) fn source_literal(word: &NativeWord) -> Option<&str> {
    if word.group().expand {
        return None;
    }
    let bytes = word
        .image()
        .bytes()
        .get(word.content_span().ok()?.as_range())?;
    if !bytes.is_ascii()
        || bytes
            .iter()
            .any(|byte| matches!(byte, b'\\' | b'$' | b'[' | b']'))
    {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

pub(super) fn current_commands(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Option<Vec<SourceCommand>> {
    let image = SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    let current_registry = analysis.resolved_registry()?;
    if !analysis.matches_original_source_image(&image, config)
        || current_registry.snapshot().semantic_key() != registry.snapshot().semantic_key()
    {
        return None;
    }
    let mut commands = BTreeMap::new();
    if !analysis.allows_retained_logical_declaration_advice() {
        use tcl_compiler::registry_invocation::InvocationWordOrigin;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let structure =
            crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
        for segment in structure.commands {
            let Some(selected) =
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, &segment,
                )
            else {
                continue;
            };
            // Hardening a separately issued sink/read needs its actual source
            // command and whole words, not complete roles for every operand.
            // Shared barriers, arity and written-origin checks remain intact.
            if !selected.matches_source(&image, config) || !selected.matches_registry(registry)
                || selected.origins().len() != segment.argv.len()
                || selected.origins().iter().enumerate().any(|(index, origin)|
                    !matches!(origin, InvocationWordOrigin::Written(written) if *written == index))
                || selected.with_source_schema(&context, |schema|
                    schema.argument_count_for_arity().is_some_and(|count|
                        schema.semantics.arity.accepts(count))) != Some(true)
            { continue; }
            let Some(head) = selected.head_source().and_then(|operand| operand.word()) else {
                continue;
            };
            let Some(arguments) = selected
                .operands()
                .iter()
                .map(|operand| operand.as_ref()?.word().cloned())
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let mut words = Vec::with_capacity(arguments.len() + 1);
            words.push(head.clone());
            words.extend(arguments);
            if words.iter().any(|word| {
                word.group().expand || word.image() != &image || word.config() != config
            }) {
                continue;
            }
            let span = Span::new(head.span().start(), words.last()?.span().end());
            commands.insert(
                span.start(),
                SourceCommand {
                    canonical: selected.command().to_owned(),
                    words,
                    span,
                },
            );
        }
    } else {
        crate::executable_regions::visit_analysis_executable_commands(
            source,
            analysis,
            &mut |segment, head, _| {
                let selected = crate::original_invocation::selected_registry_words(
                    source, analysis, segment, registry,
                );
                let canonical = if let Some(selected) = selected {
                    use tcl_compiler::registry_invocation::InvocationWordOrigin;
                    // An alias prefix or expanded operand has no simple insertion geometry.
                    if selected.origins.iter().enumerate().any(|(index, origin)|
                        !matches!(origin, InvocationWordOrigin::Written(written) if *written == index))
                    { return false; }
                    selected.command
                } else if analysis.allows_retained_logical_declaration_advice()
                    && !head.resolved.is_empty()
                {
                    let Some(spec) = super::source_action_spec(analysis, registry, head.resolved)
                    else {
                        return false;
                    };
                    spec.name.to_owned()
                } else {
                    return false;
                };
                let span = segment.execution_span(source);
                let Ok(mut plan) = tcl_lexer::native_script_words_in(image.clone(), span, config)
                else {
                    return false;
                };
                if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
                    return false;
                }
                let words = plan.commands.remove(0).words;
                if words.iter().any(|word| word.group().expand) {
                    return false;
                }
                commands.insert(
                    span.start(),
                    SourceCommand {
                        canonical,
                        words,
                        span,
                    },
                );
                false
            },
        );
    }
    Some(commands.into_values().collect())
}

pub(super) fn selected_sink<'a>(
    commands: &'a [SourceCommand],
    diagnostic: &Diagnostic,
) -> Option<&'a SourceCommand> {
    let subject = diagnostic.taint_subject.as_ref()?;
    let mut candidates = commands.iter().filter(|command| {
        let matches_sink = command.canonical == subject.sink_command()
            || subject
                .sink_command()
                .strip_prefix(&format!("{} ", command.canonical))
                .is_some_and(|suffix| {
                    command.words.get(1).and_then(source_literal) == Some(suffix)
                });
        matches_sink
            && command.span.start() <= diagnostic.span.start()
            && diagnostic.span.end() <= command.span.end()
    });
    let first = candidates.next()?;
    candidates.next().is_none().then_some(first)
}

/// One actual source read in the issuer's sink-use extent. Native reads must
/// independently belong to the current analysis; the producer's variable
/// label is never converted into a native key.
pub(super) fn selected_read(
    source: &str,
    analysis: &AnalysisResult,
    command: &SourceCommand,
    diagnostic: &Diagnostic,
) -> Option<Span> {
    let subject = diagnostic.taint_subject.as_ref()?;
    let image = SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    let mut candidates = Vec::new();
    for word in command.words.iter().skip(1) {
        let arena = word.executable_parts();
        for part in arena.all_parts() {
            let ExecutablePart::Variable { name, index } = &part.part else {
                continue;
            };
            if index.is_some()
                || part.span.start() < diagnostic.span.start()
                || part.span.end() > diagnostic.span.end()
            {
                continue;
            }
            if analysis.has_original_vendor_source_names()
                || analysis.allows_retained_logical_declaration_advice()
            {
                // Selected source syntax, rather than a physical/native name recipe.
                if image.bytes().get(name.as_range())? != subject.variable().as_bytes() {
                    continue;
                }
            } else {
                let root =
                    analysis.original_variable_root_in_source(&image, config, part.span.start())?;
                if root.part_span() != part.span {
                    return None;
                }
            }
            if !candidates.contains(&part.span) {
                candidates.push(part.span);
            }
        }
    }
    (candidates.len() == 1).then(|| candidates[0])
}

pub(super) fn helper_name_is_free(analysis: &AnalysisResult, helper: &str) -> bool {
    if analysis.has_original_vendor_source_names() {
        use tcl_syntax::naming::VendorSourceNamePurpose;
        for row in analysis.original_vendor_procedure_declarations() {
            let Some(name) = row
                .name_input()
                .literal_units(VendorSourceNamePurpose::ProcedureName)
            else {
                return false;
            };
            if name == helper.as_bytes() || name == format!("::{helper}").as_bytes() {
                return false;
            }
        }
        true
    } else if analysis.allows_retained_logical_declaration_advice() {
        // The emitted helper and its call are explicitly rooted. This
        // independently selected lexical branch asks that exact publication,
        // without borrowing an unrelated namespace's reporting tail.
        let qualified = format!("::{helper}");
        !analysis.all_procs.contains_key(&qualified)
            && !analysis.all_classes.contains_key(&qualified)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        ContextDiagnostic, context_diagnostic_actions_in_analysis, profiles_action,
    };
    use super::*;

    fn issued(source: &str, dialect: &str, code: &str) -> (AnalysisResult, ContextDiagnostic) {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        issued_in_analysis(source, analysis, None, code)
    }

    fn issued_logical(source: &str, code: &str) -> (AnalysisResult, ContextDiagnostic) {
        let mut profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-source-diagnostic-actions",
            &[],
            "Logical source diagnostic actions",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        );
        // An explicit authored C value axis in a Logical model supplies no
        // current native C or Jim naming recipe or handler entry.
        profile.runtime_base = Some(tcl_dialect::TclVersion::V8_6);
        let profile = profile.intern();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        assert!(dialect.native_name_protocol().is_none());
        let entry = tcl_compiler::command_binding::SourceAnalysisEntry {
            logical_source_input: Some(input.clone()),
            invocation_dialect: Some(dialect),
            ..Default::default()
        };
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_retained_logical_declaration_advice());
        issued_in_analysis(source, analysis, Some(&entry), code)
    }

    fn issued_in_analysis(
        source: &str,
        analysis: AnalysisResult,
        entry: Option<&tcl_compiler::command_binding::SourceAnalysisEntry>,
        code: &str,
    ) -> (AnalysisResult, ContextDiagnostic) {
        let registry = analysis.resolved_registry().unwrap();
        let profile = analysis.resolved_profile().unwrap();
        let input = analysis
            .resolved_input
            .as_ref()
            .expect("actual retained analysis input");
        let cu = tcl_compiler::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            entry,
            input.context_registry(),
        )
        .with_interprocedural(registry, Some(profile));
        let diagnostics =
            tcl_compiler::compiler_checks::run_all_checks(&cu, registry, Some(profile));
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_str() == code)
            .unwrap_or_else(|| panic!("missing {code}: {diagnostics:?}"));
        let data = ContextDiagnosticData::from_compiler_diagnostic(diagnostic)
            .expect("original issuer context");
        let wire = data.to_value();
        let data = ContextDiagnosticData::from_value(&wire, code).expect("retained wire subject");
        let diagnostic = ContextDiagnostic {
            code: code.to_owned(),
            message: diagnostic.message.clone(),
            data: Some(data),
            range: crate::definition::span_to_range(
                source,
                &tcl_lexer::LineIndex::new(source),
                diagnostic.span,
            ),
        };
        (analysis, diagnostic)
    }

    fn actions(
        source: &str,
        analysis: &AnalysisResult,
        diagnostic: &ContextDiagnostic,
    ) -> Vec<super::super::CodeAction> {
        context_diagnostic_actions_in_analysis(
            source,
            analysis,
            analysis.resolved_registry().unwrap(),
            std::slice::from_ref(diagnostic),
        )
    }

    #[test]
    fn logical_source_action_commands_use_actual_body_availability_and_script_purpose() {
        // naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        // Source-command geometry only; no sink, effect permission or Native edit grant.
        fn reference_only(
            _args: tcl_registry::InvocationArguments<'_>,
        ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
            vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
        }
        let source = "gated-body {set nested VALUE}; reference-script {set reference VALUE}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let surface = registry.get("dict").unwrap().surface;
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-body",
            surface,
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "reference-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            script_timing_resolver: Some(reference_only),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let store = std::sync::Arc::new(registry);
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = std::sync::Arc::new(
                tcl_registry::model::ingress::static_context_for(environment)
                    .with_command_store(std::sync::Arc::clone(&store)),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let mut analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            assert!(analysis.allows_retained_logical_declaration_advice());
            let commands = current_commands(source, &analysis, &store).unwrap();
            let nested = commands
                .iter()
                .find(|command| &source[command.span.as_range()] == "set nested VALUE");
            assert_eq!(
                nested.is_some(),
                available,
                "retained {environment} body availability"
            );
            if let Some(nested) = nested {
                assert_eq!(nested.canonical, "set");
                assert_eq!(nested.words.len(), 3);
            }
            assert!(
                !commands
                    .iter()
                    .any(|command| &source[command.span.as_range()] == "set reference VALUE")
            );
            assert!(current_commands(&format!("{source}# changed"), &analysis, &store).is_none());
            analysis.resolved_input = None;
            assert!(current_commands(source, &analysis, &store).is_none());
        }
    }

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_diagnostic_actions_keep_typed_sink_and_source_when_prose_changes() {
        let source = "set x [gets stdin]\nset out [subst $x]\n";
        let (analysis, mut diagnostic) = issued_logical(source, "T100");
        let expected = actions(source, &analysis, &diagnostic);
        assert_eq!(expected.len(), 1, "{expected:?}");
        assert_eq!(expected[0].edits[0].new_text, " -nocommands");
        diagnostic.message = "Tainted $other flows into eval; presentation changed".to_owned();
        assert_eq!(actions(source, &analysis, &diagnostic), expected);
        diagnostic.data = None;
        assert!(actions(source, &analysis, &diagnostic).is_empty());
    }

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_diagnostic_actions_refuse_stale_source_configuration_registry_and_range() {
        let source = "set x [gets stdin]\nsubst $x\n";
        let (mut analysis, diagnostic) = issued_logical(source, "T100");
        assert!(!actions(source, &analysis, &diagnostic).is_empty());
        assert!(actions(&format!("{source}# changed\n"), &analysis, &diagnostic).is_empty());
        let mut moved = diagnostic.clone();
        moved.range.start_character += 1;
        assert!(actions(source, &analysis, &moved).is_empty());
        let mut forged = diagnostic.data.as_ref().unwrap().to_value();
        forged["kind"] = serde_json::json!("arbitrary source subject");
        assert!(ContextDiagnosticData::from_value(&forged, "T100").is_none());
        assert!(
            ContextDiagnosticData::from_value(
                &diagnostic.data.as_ref().unwrap().to_value(),
                "T101"
            )
            .is_none()
        );
        assert!(
            context_diagnostic_actions_in_analysis(
                source,
                &analysis,
                crate::registry_for_dialect("tcl9.1"),
                std::slice::from_ref(&diagnostic)
            )
            .is_empty()
        );
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(actions(source, &analysis, &diagnostic).is_empty());
    }

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_vendor_taint_helpers_use_declarations_instead_of_comments_or_proc_text() {
        let body = "when HTTP_REQUEST {set x [HTTP::query]; HTTP::respond 200 content $x}\n";
        let commented = format!("# proc html_encode is only a comment\n{body}");
        let (analysis, diagnostic) = issued(&commented, "f5-irules", "IRULE3001");
        let registry = analysis.resolved_registry().unwrap();
        let data = diagnostic.data.as_ref().unwrap();
        assert!(
            data.matches(&commented, &analysis, registry, &diagnostic),
            "the actual typed diagnostic retains its current whole source"
        );
        let commands = current_commands(&commented, &analysis, registry).unwrap();
        let issuer = data.diagnostic().unwrap();
        let sink = selected_sink(&commands, issuer).unwrap_or_else(|| {
            let geometry: Vec<_> = commands
                .iter()
                .map(|row| (row.canonical.as_str(), row.span.start(), row.span.end()))
                .collect();
            panic!("the guarded current sink source is absent: {geometry:?}");
        });
        assert!(
            selected_read(&commented, &analysis, sink, issuer).is_some(),
            "the typed sink subject retains its exact original variable read"
        );
        let offered = actions(&commented, &analysis, &diagnostic);
        assert_eq!(offered.len(), 1, "{offered:?}");
        assert!(
            offered[0]
                .edits
                .iter()
                .any(|edit| edit.new_text.starts_with("proc html_encode "))
        );
        assert!(
            offered[0]
                .edits
                .iter()
                .any(|edit| edit.new_text == "[::html_encode $x]")
        );
        let occupied = format!("proc {{html_encode}} {{value}} {{return $value}}\n{body}");
        let (analysis, diagnostic) = issued(&occupied, "f5-irules", "IRULE3001");
        assert!(actions(&occupied, &analysis, &diagnostic).is_empty());
    }

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_native_taint_wrappers_do_not_infer_missing_evaluation_permission() {
        let source = "set x [gets stdin]\nputs $x\n";
        let (analysis, diagnostic) = issued(source, "tcl8.6", "T101");
        assert!(diagnostic.data.is_some());
        assert!(actions(source, &analysis, &diagnostic).is_empty());
    }

    // Implementation contract: naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    #[test]
    fn original_bootstrap_actions_use_current_selected_handler_and_payload_metadata() {
        for (source, code) in [
            (
                "when HTTP_REQUEST_DATA {set value [HTTP::payload]}\n",
                "IRULE1005",
            ),
            (
                "when HTTP_RESPONSE {set marker 😀; HTTP::payload}\n",
                "IRULE1006",
            ),
        ] {
            let (analysis, mut diagnostic) = issued(source, "f5-irules", code);
            let expected = actions(source, &analysis, &diagnostic);
            assert!(!expected.is_empty(), "{code}");
            assert!(
                expected
                    .iter()
                    .all(|action| action.edits[0].new_text.contains("HTTP::collect"))
            );
            diagnostic.message =
                "TCP::collect in an unrelated event; counterfactual prose".to_owned();
            assert_eq!(actions(source, &analysis, &diagnostic), expected);
            assert!(
                context_diagnostic_actions_in_analysis(
                    source,
                    &analysis,
                    crate::registry_for_dialect("tcl8.6"),
                    std::slice::from_ref(&diagnostic)
                )
                .is_empty()
            );
        }
    }

    // Implementation contract: naming.consumer.original-profile-source-advice
    // docs/design/analysis/name-resolution-proofs/original-profile-source-advice.md
    #[test]
    fn original_profile_actions_use_selected_context_and_ignore_reporting_names() {
        let source = "when HTTP_REQUEST {HTTP::respond 200 content ok}\n";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let registry = analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .shared_registry();
        let expected =
            profiles_action(source, &analysis, &registry).expect("actual HTTP source metadata");
        for invocation in &mut analysis.command_invocations {
            invocation.name = "SSL::counterfactual".to_owned();
        }
        analysis.all_procs.clear();
        assert_eq!(
            profiles_action(source, &analysis, &registry),
            Some(expected)
        );
        assert!(profiles_action(&format!("{source}# changed"), &analysis, &registry).is_none());
        assert!(
            profiles_action(source, &analysis, crate::registry_for_dialect("f5-tmsh")).is_none()
        );
    }
    #[test]
    fn original_source_diagnostic_advice_shares_guarded_schema_and_whole_word_geometry() {
        // Implementation contract: naming.consumer.original-profile-source-advice
        // docs/design/analysis/name-resolution-proofs/original-profile-source-advice.md
        use super::current_commands;
        let source = "when HTTP_REQUEST {HTTP::respond 200 content ok}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let registry = analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .shared_registry();
        let selected = current_commands(source, &analysis, &registry).unwrap();
        let response = selected
            .iter()
            .find(|command| command.canonical == "HTTP::respond")
            .expect("actual guarded source metadata");
        assert_eq!(response.words.len(), 4);
        assert_eq!(
            source.get(response.words[0].span().as_range()),
            Some("HTTP::respond")
        );
        assert!(
            response
                .words
                .iter()
                .all(|word| word.image() == &tcl_lexer::SourceImage::document(source))
        );
        analysis.all_procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        assert!(
            current_commands(source, &analysis, &registry)
                .unwrap()
                .iter()
                .any(|command| command.canonical == "HTTP::respond")
        );
        assert!(current_commands(&format!("{source}# changed"), &analysis, &registry).is_none());
        assert!(
            current_commands(source, &analysis, crate::registry_for_dialect("f5-tmsh")).is_none()
        );
        let source = "proc HTTP::respond {args} {return custom}; when HTTP_REQUEST {HTTP::respond 200 content ok}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let registry = analysis.resolved_registry().unwrap();
        assert!(
            !current_commands(source, &analysis, registry)
                .unwrap()
                .iter()
                .any(|command| command.canonical == "HTTP::respond"),
            "definite original source declaration barrier"
        );
        // TMM excludes rename, so this head cannot issue a definite deletion.
        let source = "rename HTTP::respond {}; when HTTP_REQUEST {HTTP::respond 200 content ok}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let registry = analysis.resolved_registry().unwrap();
        let (metadata, _) =
            tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source,
                &analysis,
                u32::try_from(source.rfind("HTTP::respond").unwrap()).unwrap(),
            )
            .expect("conditional source advice retains the unavailable transition obligation");
        assert!(metadata.authored_barriers().has_unknown_transitions());
        assert!(!metadata.obligations().is_empty());
        assert!(
            current_commands(source, &analysis, registry)
                .unwrap()
                .iter()
                .any(|command| command.canonical == "HTTP::respond")
        );
        // This independent loader context actually admits rename. Its selected
        // transition blocks advice for the original source coordinate.
        for source in [
            "rename source moved; source selected.tcl",
            "rename source {}; source selected.tcl",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-iapps");
            let registry = analysis.resolved_registry().unwrap();
            assert!(
                !current_commands(source, &analysis, registry)
                    .unwrap()
                    .iter()
                    .any(|command| command.canonical == "source"),
                "selected source transition: {source}"
            );
        }
    }

    #[test]
    fn original_logical_source_commands_use_actual_availability_over_catalogue_profile() {
        // naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        // A source metadata discriminator, not a loaded/native command proof.
        let source = "dict create key value";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let catalogue =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        for (actual, available) in [("tcl8.6", true), ("tcl8.4", false)] {
            let context = tcl_registry::model::ingress::resolve_environment(actual)
                .default_context_registry()
                .with_command_store(catalogue.commands().clone());
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(context),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let analysis = tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            assert!(analysis.allows_retained_logical_declaration_advice());
            let commands = current_commands(source, &analysis, catalogue.commands()).unwrap();
            assert_eq!(
                commands.iter().any(|command| command.canonical == "dict"),
                available,
                "{actual}"
            );
        }
    }

    #[test]
    fn original_logical_helper_collision_uses_the_emitted_root_publication() {
        // Implementation contract: naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        let registry = crate::registry_for_dialect("tcl8.6");
        let point =
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "explicit-lexical-helper",
            &[],
            "Logical helper source advice",
            point,
        )
        .intern();
        let context = crate::context_for_dialect_profile(profile)
            .with_command_store(registry.snapshot().shared_registry());
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let analyse = |source: &str| {
            tcl_compiler::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name)
        };
        let source = "namespace eval N {proc html_encode {value} {return $value}}";
        let analysis = analyse(source);
        assert!(analysis.allows_retained_logical_declaration_advice());
        assert!(super::helper_name_is_free(&analysis, "html_encode"));
        let mut missing_input = analysis.clone();
        missing_input.resolved_input = None;
        assert!(!missing_input.allows_lexical_declaration_advice());
        assert!(!missing_input.allows_retained_logical_declaration_advice());
        assert!(!super::helper_name_is_free(&missing_input, "html_encode"));
        assert!(current_commands(source, &missing_input, registry).is_none());
        let source = "proc html_encode {value} {return $value}";
        let analysis = analyse(source);
        assert!(!super::helper_name_is_free(&analysis, "html_encode"));
        let native = tcl_compiler::analyser::Analyser::new().analyse("", "tcl8.6");
        assert!(!super::helper_name_is_free(&native, "html_encode"));
    }
}
