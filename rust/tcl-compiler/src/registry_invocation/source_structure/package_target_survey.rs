// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed source-advertisement survey: hypothetical target release, not runtime facts.
use super::*;
use tcl_registry::ArgRole;
use tcl_syntax::expr::{ExprNode, UnaryOp};

/// Fixed declared-source survey over one immutable source and supplied Registry.
/// Its initial core provision is an explicit hypothetical model parameter;
/// actual analysis/provision queries remain separate and unknown.
#[derive(Debug)]
pub struct OriginalPackageIndexSourceSurvey {
    source: std::sync::Arc<OriginalSourceRegistryContext>,
    ancestors: Vec<(
        std::sync::Arc<OriginalSourceRegistryContext>,
        tcl_lexer::NativeScriptCommandWords,
    )>,
}

/// Exact Logical source-world control premise at the retained original site.
/// This models conditional source control only, never argv success or Native dispatch.
#[derive(Debug)]
pub struct OriginalPackageIndexControl {
    words: OriginalRegistryWords,
    completion: tcl_registry::registry::InvocationCompletion,
    _model: crate::command_binding::OriginalPackageIndexSourceModel,
    _ancestors: Vec<OriginalPackageIndexControl>,
}
impl OriginalPackageIndexControl {
    /// Selected source descriptor and full original operands.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Completion shape from that same selected descriptor and argument dialect.
    #[must_use]
    pub const fn completion(&self) -> tcl_registry::registry::InvocationCompletion {
        self.completion
    }
}

/// Conditional selected source advertisement, independently of argv completion
/// or complete package inventory. This never proves a physical registration.
#[derive(Debug)]
pub struct OriginalPackageIndexAdvertisement {
    words: OriginalRegistryWords,
    name: usize,
    version: usize,
    body: usize,
}
impl OriginalPackageIndexAdvertisement {
    /// Exact selected source words, retaining image, input and source advice.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Original static Unicode package value when available, without Native key authority.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.value(self.name)
    }
    /// Original static Unicode version value when available.
    #[must_use]
    pub fn version(&self) -> Option<String> {
        self.value(self.version)
    }
    /// Effective post-head deferred script ordinal from the selected descriptor.
    #[must_use]
    pub const fn body_argument(&self) -> usize {
        self.body
    }
    fn value(&self, argument: usize) -> Option<String> {
        String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            self.words.original_argument_word(argument)?,
        )?)
        .ok()
    }
}

/// Actually entered script plus its inherited immutable source-survey context.
/// The script keeps its original contiguous or materialised mapping and channel.
#[derive(Debug)]
pub struct OriginalPackageIndexRetainedBody {
    survey: OriginalPackageIndexSourceSurvey,
    script: crate::command_binding::ExecutedScriptSource,
    _parent_words: OriginalRegistryWords,
}
impl OriginalPackageIndexRetainedBody {
    /// Child context borrowed from actual retained source points, not reanalysis.
    #[must_use]
    pub fn survey(&self) -> &OriginalPackageIndexSourceSurvey {
        &self.survey
    }
    /// Exact interpreter-owned script carrier and mapping.
    #[must_use]
    pub fn script(&self) -> &crate::command_binding::ExecutedScriptSource {
        &self.script
    }
}

/// Original query geometry and independently retained pre-query model history.
/// This supplies only a condition for the caller's hypothetical release survey.
#[derive(Debug)]
pub struct OriginalPackageIndexTargetGuard {
    requirements: Vec<String>,
    release_arity: Vec<(tcl_dialect::TclVersion, bool)>,
    negated: bool,
    subject: String,
    _models: Vec<crate::command_binding::OriginalPackageIndexSourceModel>,
    _words: Vec<OriginalRegistryWords>,
}
impl OriginalPackageIndexTargetGuard {
    /// Original static requirement values; release/patch uncertainty is evaluated
    /// by the existing shared release relation, never a pinned build version.
    #[must_use]
    pub fn requirements(&self) -> &[String] {
        &self.requirements
    }
    /// Whether `target`'s own Registry form for the selected query accepts
    /// its original argv count. `Some(false)`: the query raises instead of
    /// answering (Tcl 8.4's `package vsatisfies` takes exactly one
    /// requirement). `None`: the Registry left the count undecided.
    #[must_use]
    pub fn argv_accepted_in(&self, target: tcl_dialect::TclVersion) -> Option<bool> {
        self.release_arity
            .iter()
            .find_map(|&(release, accepted)| (release == target).then_some(accepted))
    }
    /// Whether the complete original expression negates the selected query.
    #[must_use]
    pub const fn negated(&self) -> bool {
        self.negated
    }
    /// The supplied target model declares this subject as its interpreter release.
    /// False withholds the model; it does not prove package absence at runtime.
    #[must_use]
    pub fn subject_is_interpreter_release_in(&self, target: tcl_dialect::TclVersion) -> bool {
        target.core_provided_packages().iter().any(|row| {
            row.role == tcl_dialect::CorePackageRole::InterpreterRelease && row.name == self.subject
        })
    }
}

impl OriginalPackageIndexSourceSurvey {
    /// Capture an explicit fixed survey, preserving the actual whole grammar
    /// and supplied Registry generation. A detached body is not a fresh root.
    #[must_use]
    pub fn capture(
        source: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let generation = crate::environment_ingress::context_for_profile(profile);
        let context = std::sync::Arc::new(
            generation.with_command_store(registry.snapshot().shared_registry()),
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        Self {
            source: std::sync::Arc::new(
                OriginalSourceRegistryContext::capture_package_index_survey(source, input),
            ),
            ancestors: Vec::new(),
        }
    }

    /// Exact complete root image retained by this source-model purpose.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        self.source.source_image()
    }
    /// Same immutable complete source grammar and Registry availability.
    #[must_use]
    pub fn editing_input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        self.source.editing_input()
    }

    /// Actual Logical source-advice words; no Native string recipe or promotion
    /// of the general facade's false lookup-preservation flag occurs.
    #[must_use]
    pub fn words(
        &self,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        logical_source_registry_words_in_original_image(
            &self.source.image,
            &self.source.realm,
            &self.source.input,
            segment,
        )
    }

    fn command_words(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<OriginalRegistryWords> {
        let image = &self.source.image;
        let config = self.source.input.lexer_config();
        if command
            .words
            .iter()
            .any(|word| word.image() != image || word.config() != config)
        {
            return None;
        }
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            image.try_text().ok()?.get(command.span.as_range())?,
            command.span.start(),
            config.nested(),
        );
        let [segment] = segments.as_slice() else {
            return None;
        };
        let words = self.words(segment)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return None;
        };
        (advice.original_words() == command.words.as_slice()).then_some(words)
    }
    fn model(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        subject: &str,
    ) -> Option<crate::command_binding::OriginalPackageIndexSourceModel> {
        let words = self.command_words(command)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return None;
        };
        self.source
            .realm
            .source_bindings_ref()
            .original_package_index_model_at(
                &self.source.input,
                &self.source.image,
                command,
                Some(subject),
                advice,
            )
    }

    /// Retain genuine source lookup/history and selected completion together.
    /// Unknown operands, prior effects or source-entry mismatches cannot be
    /// repaired by a nominal command-name fallback.
    #[must_use]
    pub fn control(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<OriginalPackageIndexControl> {
        let words = self.command_words(command)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return None;
        };
        let model = self
            .source
            .realm
            .source_bindings_ref()
            .original_package_index_model_at(
                &self.source.input,
                &self.source.image,
                command,
                None,
                advice,
            )?;
        let completion = words
            .with_source_schema(self.source.input.borrowed_context_registry(), |schema| {
                schema.authored_source_invocation_completion()
            })?;
        let ancestors = self
            .ancestors
            .iter()
            .map(|(source, command)| {
                Self {
                    source: std::sync::Arc::clone(source),
                    ancestors: Vec::new(),
                }
                .control(command)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(OriginalPackageIndexControl {
            words,
            completion,
            _model: model,
            _ancestors: ancestors,
        })
    }

    /// Retain a selected source declaration under this explicit hypothetical
    /// advertisement purpose. Unsafe argv may remain undiscovered/incomplete;
    /// this receipt does not promote lookup or execution flags.
    #[must_use]
    pub fn advertisement(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<OriginalPackageIndexAdvertisement> {
        let words = self.command_words(command)?;
        let (name, version, body) = words
            .with_source_schema(self.source.input.borrowed_context_registry(), |schema| {
                schema.authored_source_package_registration()
            })??;
        if command.words.iter().any(|word| word.group().expand)
            || !words.origins().iter().enumerate().all(|(ordinal, origin)| {
                matches!(origin, crate::registry_invocation::InvocationWordOrigin::Written(written) if *written == ordinal)
            }) { return None; }
        Some(OriginalPackageIndexAdvertisement {
            words,
            name,
            version,
            body,
        })
    }

    /// Retain an actually entered Body operand through the source interpreter's
    /// unanimous evaluated-script carrier. This never captures detached text.
    #[must_use]
    pub fn retained_body(
        &self,
        parent: &tcl_lexer::NativeScriptCommandWords,
        argument: usize,
    ) -> Option<OriginalPackageIndexRetainedBody> {
        let words = self.command_words(parent)?;
        if words
            .roles()?
            .iter()
            .filter(|(index, role)| *index == argument && *role == ArgRole::Body)
            .count()
            != 1
        {
            return None;
        }
        let original = words.original_argument_word(argument)?;
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(original)?;
        let operand = words.operands().get(argument)?.as_ref()?;
        let script = self
            .source
            .realm
            .executed_script_for_word(operand.span())?
            .clone();
        if script.text.bytes() != value.as_slice() {
            return None;
        }
        let source = self.retained_body_source(&script)?;
        let mut ancestors = self.ancestors.clone();
        ancestors.push((std::sync::Arc::clone(&self.source), parent.clone()));
        Some(OriginalPackageIndexRetainedBody {
            survey: Self { source, ancestors },
            script,
            _parent_words: words,
        })
    }

    /// The source context an entered body's survey borrows. A body of this
    /// interpretation's own root origin selects these very bindings
    /// (`selected_source` returns them unchanged), so it shares this context,
    /// its realm and its prepared source advice instead of rebuilding them
    /// once per nesting level. A derived origin selects its own bindings.
    fn retained_body_source(
        &self,
        script: &crate::command_binding::ExecutedScriptSource,
    ) -> Option<std::sync::Arc<OriginalSourceRegistryContext>> {
        let image = script.origin.source_image();
        let config = self.source.input.lexer_config();
        if self.source.realm.source_bindings_ref().source_origin() == Some(&script.origin) {
            return (self.source.image == *image
                && self
                    .source
                    .realm
                    .matches_original_source_image(image, config)
                && self
                    .source
                    .realm
                    .matches_resolved_analysis_input(&self.source.input))
            .then(|| std::sync::Arc::clone(&self.source));
        }
        let bindings = self
            .source
            .realm
            .source_bindings_ref()
            .selected_source(script)?;
        let realm = crate::realm::original_logical_realm_from_source_bindings(&bindings)?;
        if !realm.matches_original_source_image(image, config)
            || !realm.matches_resolved_analysis_input(&self.source.input)
        {
            return None;
        }
        Some(std::sync::Arc::new(OriginalSourceRegistryContext {
            image: image.clone(),
            input: self.source.input.clone(),
            realm,
        }))
    }

    /// Whole original braced Expr operand -> actual retained query descriptors,
    /// exact nested source commands and untouched hypothetical-subject history.
    /// Require is deliberately not a read-only substitute for provide.
    #[must_use]
    pub fn target_guard(
        &self,
        parent: &tcl_lexer::NativeScriptCommandWords,
        argument: usize,
    ) -> Option<OriginalPackageIndexTargetGuard> {
        use tcl_registry::source_navigation::SourcePackageQuery;
        let words = self.command_words(parent)?;
        let context = self.source.input.context_registry();
        let expression = words.original_argument_word(argument)?;
        if expression.group().expand
            || expression.group().kind != tcl_lexer::WordKind::Braced
            || words
                .roles()?
                .iter()
                .filter(|(index, role)| *index == argument && *role == ArgRole::Expr)
                .count()
                != 1
        {
            return None;
        }
        let content = expression.content_span().ok()?;
        let text = self.source.image.try_text().ok()?.get(content.as_range())?;
        let mut syntax = tcl_syntax::expr::parser::ExprParseContext::for_profile(
            self.source.input.analyser_profile(),
        );
        syntax.lexer_grammar = self
            .source
            .input
            .lexer_config()
            .grammar_over(self.source.input.analyser_profile().grammar);
        // The existing source parser returns Raw for unknown lexing, budget,
        // syntax rejection or unconsumed tokens. No Native syntax is selected.
        let tree = tcl_syntax::expr::parser::parse_expr_with_syntax_context(text, &syntax);
        let mut node = &tree;
        let mut negated = false;
        while let ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } = node
        {
            negated = !negated;
            node = operand;
        }
        let ExprNode::Command {
            start,
            end,
            text: command_text,
        } = node
        else {
            return None;
        };
        let extent = tcl_syntax::expr::expression_span(text, *start, *end)?;
        if text.get(extent.as_range()) != Some(command_text.as_str()) {
            return None;
        }
        let scripts =
            words.source_expression_script_bodies_at(&self.source.input, content.start())?;
        let [script] = scripts.as_slice() else {
            return None;
        };
        let expected = tcl_lexer::Span::new(
            content
                .start()
                .checked_add(extent.start())?
                .checked_add(1)?,
            content.start().checked_add(extent.end())?.checked_sub(1)?,
        );
        if script.original_container() != expression
            || script.content_span() != expected
            || !script.matches_source(&self.source.image, self.source.input.lexer_config())
        {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.source.image.clone(),
            expected,
            self.source.input.lexer_config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let [outer] = plan.commands.as_slice() else {
            return None;
        };
        let outer_words = self.command_words(outer)?;
        let (outer_query, release_arity) =
            outer_words.with_source_schema(&context, |schema| {
                let decided = tcl_dialect::TclVersion::ALL
                    .into_iter()
                    .filter_map(|release| {
                        tcl_registry::InvocationDialect::for_version(release)
                            .authoring_query()
                            .and_then(|query| schema.arity_accepts_under(&query))
                            .map(|accepted| (release, accepted))
                    })
                    .collect::<Vec<_>>();
                schema
                    .authored_source_package_query()
                    .map(|query| (query, decided))
            })??;
        if outer_query.query != SourcePackageQuery::VersionSatisfies {
            return None;
        }
        let subject_word = outer_words.original_argument_word(outer_query.arguments.start)?;
        let inner = original_single_source_command_substitution(subject_word)?;
        let inner_words = self.command_words(inner.command())?;
        let inner_query = inner_words
            .with_source_schema(&context, |schema| schema.authored_source_package_query())??;
        if inner_query.query != SourcePackageQuery::ProvidedVersion {
            return None;
        }
        let subject =
            String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                inner_words.original_argument_word(inner_query.arguments.start)?,
            )?)
            .ok()?;
        if ![
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ]
        .iter()
        .any(|target| {
            target.core_provided_packages().iter().any(|row| {
                row.role == tcl_dialect::CorePackageRole::InterpreterRelease && row.name == subject
            })
        }) {
            return None;
        }
        let requirements = (outer_query.arguments.start.checked_add(1)?..outer_query.arguments.end)
            .map(|ordinal| {
                String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                    outer_words.original_argument_word(ordinal)?,
                )?)
                .ok()
            })
            .collect::<Option<Vec<_>>>()?;
        let models = vec![
            self.model(parent, &subject)?,
            self.model(outer, &subject)?,
            self.model(inner.command(), &subject)?,
        ];
        if models
            .iter()
            .skip(1)
            .any(|model| !model.same_source_context(&models[0]))
        {
            return None;
        }
        let mut models = models;
        for (source, command) in &self.ancestors {
            let ancestor = Self {
                source: std::sync::Arc::clone(source),
                ancestors: Vec::new(),
            };
            models.push(ancestor.model(command, &subject)?);
        }
        Some(OriginalPackageIndexTargetGuard {
            requirements,
            release_arity,
            negated,
            subject,
            _models: models,
            _words: vec![words, outer_words, inner_words],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guard(source: &str) -> Option<OriginalPackageIndexTargetGuard> {
        let registry = CommandRegistry::build_default();
        let config =
            tcl_lexer::LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
        let survey = OriginalPackageIndexSourceSurvey::capture(source, &registry, config);
        let plan = tcl_lexer::native_script_words_in(
            survey.source_image().clone(),
            tcl_lexer::Span::new(0, source.len() as u32),
            config,
        )
        .unwrap();
        survey.target_guard(plan.commands.last().unwrap(), 0)
    }

    #[test]
    fn logical_target_guard_uses_actual_queries_without_native_string_protocol() {
        let receipt =
            guard("if {![package vsatisfies [package provide Tcl] 8.5 9]} {return}").unwrap();
        assert!(receipt.negated());
        assert_eq!(receipt.requirements(), &["8.5".to_owned(), "9".to_owned()]);
        assert!(receipt.subject_is_interpreter_release_in(tcl_dialect::TclVersion::V8_6));
        assert_eq!(
            receipt.argv_accepted_in(tcl_dialect::TclVersion::V8_4),
            Some(false)
        );
        assert_eq!(
            receipt.argv_accepted_in(tcl_dialect::TclVersion::V8_5),
            Some(true)
        );
        assert_eq!(
            receipt.argv_accepted_in(tcl_dialect::TclVersion::V9_1),
            Some(true)
        );
        let lower = guard("if {[package vsatisfies [package provide tcl] 9.0-]} {return}").unwrap();
        assert_eq!(
            lower.argv_accepted_in(tcl_dialect::TclVersion::V8_4),
            Some(true)
        );
        assert!(!lower.subject_is_interpreter_release_in(tcl_dialect::TclVersion::V8_6));
        assert!(lower.subject_is_interpreter_release_in(tcl_dialect::TclVersion::V9_0));
    }

    #[test]
    fn target_model_retires_on_prior_source_and_package_history() {
        for prefix in [
            "package provide Tcl 7.0;",
            "package forget Tcl;",
            "package ifneeded Tcl 1.0 {source replacement.tcl};",
            "package unknown handler;",
            "mystery;",
            "source other.tcl;",
            "rename package oldPackage;",
            "proc package args {return 1};",
        ] {
            let source = format!(
                "{prefix} if {{[package vsatisfies [package provide Tcl] 8.5]}} {{return}}"
            );
            assert!(guard(&source).is_none(), "{source}");
        }
    }

    #[test]
    fn source_query_algebra_refuses_other_subjects_loads_and_extra_expression_work() {
        for source in [
            "if {[package vsatisfies [package provide TclOO] 1.0]} {return}",
            "if {[package vsatisfies [package require Tcl] 8.5]} {return}",
            "if {[package vsatisfies [package provide Tcl] 8.5] && [mystery]} {return}",
            "if {[package vsatisfies [package provide Tcl] 8.5] trailing} {return}",
            "if {[package vsatisfies [package provide Tcl] $requirement]} {return}",
        ] {
            assert!(guard(source).is_none(), "{source}");
        }
    }

    #[test]
    fn survey_source_advice_does_not_promote_the_general_lookup_flag() {
        let source = "if {1} {return}";
        let registry = CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::default();
        let survey = OriginalPackageIndexSourceSurvey::capture(source, &registry, config);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = survey.words(&segments[0]).unwrap();
        assert!(matches!(
            words.source(),
            OriginalRegistrySource::SourceTransitions(_)
        ));
        assert!(!words.operands_preserve_source_lookup());
        assert_eq!(survey.editing_input().lexer_config(), config);
        assert!(
            tcl_registry::InvocationDialect::of_profile(survey.editing_input().unit_profile())
                .native_source_string_protocol()
                .is_none()
        );
    }
}

#[cfg(test)]
mod retained_body_tests {
    use super::*;
    fn capture(
        source: &str,
    ) -> (
        OriginalPackageIndexSourceSurvey,
        tcl_lexer::NativeScriptCommandWords,
    ) {
        let config =
            tcl_lexer::LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
        let survey = OriginalPackageIndexSourceSurvey::capture(
            source,
            &CommandRegistry::build_default(),
            config,
        );
        let plan = tcl_lexer::native_script_words_in(
            survey.source_image().clone(),
            tcl_lexer::Span::new(0, source.len() as u32),
            config,
        )
        .unwrap();
        let command = plan.commands.last().unwrap().clone();
        (survey, command)
    }
    #[test]
    fn retained_body_preserves_actual_script_mapping_and_input() {
        for source in ["if 1 {return}", "if 1 \"ret\\u0075rn\""] {
            let (survey, command) = capture(source);
            let body = survey.retained_body(&command, 1).unwrap();
            assert_eq!(body.script().try_text().unwrap(), "return");
            assert_eq!(body.survey().editing_input(), survey.editing_input());
            assert_eq!(
                body.survey().source_image(),
                body.script().origin.source_image()
            );
            let expected = survey
                .source
                .realm
                .executed_script_for_word(
                    survey.command_words(&command).unwrap().operands()[1]
                        .as_ref()
                        .unwrap()
                        .span(),
                )
                .unwrap();
            assert_eq!(body.script(), expected);
        }
    }
    /// The survey's conditional source control enters the selected body; the
    /// general Logical context over the very same input does not. Both share
    /// one generation, so they would share an authored-interpretation memo
    /// key; each capture order is exercised.
    #[test]
    fn conditional_source_control_stays_inside_the_survey() {
        let source = "if 1 {return}";
        let registry = CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        for survey_first in [true, false] {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(
                    crate::environment_ingress::context_for_profile(profile)
                        .with_command_store(registry.snapshot().shared_registry()),
                ),
                config,
            );
            let (survey, general) = if survey_first {
                let survey = OriginalSourceRegistryContext::capture_package_index_survey(
                    source,
                    input.clone(),
                );
                (
                    survey,
                    OriginalSourceRegistryContext::capture(source, input),
                )
            } else {
                let general = OriginalSourceRegistryContext::capture(source, input.clone());
                (
                    OriginalSourceRegistryContext::capture_package_index_survey(source, input),
                    general,
                )
            };
            let plan = tcl_lexer::native_script_words_in(
                survey.image.clone(),
                tcl_lexer::Span::new(0, source.len() as u32),
                config,
            )
            .unwrap();
            let survey = OriginalPackageIndexSourceSurvey {
                source: std::sync::Arc::new(survey),
                ancestors: Vec::new(),
            };
            let span = survey.command_words(&plan.commands[0]).unwrap().operands()[1]
                .as_ref()
                .unwrap()
                .span();
            assert!(
                survey.source.realm.executed_script_for_word(span).is_some(),
                "survey first: {survey_first}"
            );
            assert!(
                general.realm.executed_script_for_word(span).is_none(),
                "survey first: {survey_first}"
            );
        }
    }

    /// tclsh 8.4.20 to 9.1.0: `if 1 {set ::ran 1} else`, `… elseif` and
    /// `… elseif 0` raise `wrong # args` before any clause runs, so `::ran`
    /// stays unset; `… else {}` runs the first body. A malformed chain
    /// therefore enters no body, while the complete chain does.
    #[test]
    fn malformed_first_match_chains_enter_no_body() {
        for (source, entered) in [
            ("if 1 {return} else", false),
            ("if 1 {return} elseif", false),
            ("if 1 {return} elseif 0", false),
            ("if 1 {return} else {}", true),
        ] {
            let (survey, command) = capture(source);
            assert_eq!(
                survey.retained_body(&command, 1).is_some(),
                entered,
                "{source}"
            );
        }
    }

    #[test]
    fn retained_body_refuses_inert_dynamic_and_foreign_vectors() {
        for source in ["set x {return}", "if 1 $body"] {
            let (survey, command) = capture(source);
            assert!(survey.retained_body(&command, 1).is_none());
        }
        let (survey, _) = capture("if 1 {return}");
        let (_, foreign) = capture("if 1 {break}");
        assert!(survey.retained_body(&foreign, 1).is_none());
    }
}

#[cfg(test)]
mod advertisement_tests {
    use super::*;
    fn advertisement(source: &str) -> Option<OriginalPackageIndexAdvertisement> {
        let config = tcl_lexer::LexerConfig::default();
        let survey = OriginalPackageIndexSourceSurvey::capture(
            source,
            &CommandRegistry::build_default(),
            config,
        );
        let plan = tcl_lexer::native_script_words_in(
            survey.source_image().clone(),
            tcl_lexer::Span::new(0, source.len() as u32),
            config,
        )
        .ok()?;
        survey.advertisement(plan.commands.last()?)
    }
    #[test]
    fn selected_advertisement_retains_original_unicode_and_deferred_geometry() {
        let receipt = advertisement("package ifneeded café 1.0 [list source $dir/p.tcl]").unwrap();
        assert_eq!(receipt.name().as_deref(), Some("café"));
        assert_eq!(receipt.version().as_deref(), Some("1.0"));
        assert_eq!(receipt.body_argument(), 3);
        assert!(!receipt.words().operands_preserve_source_lookup());
    }
    #[test]
    fn selected_advertisement_refuses_shadow_expansion_and_inert_data() {
        for source in [
            "proc package args {}; package ifneeded p 1.0 {}",
            "package {*}{ifneeded p 1.0 {}}",
            "set x {package ifneeded p 1.0 {}}",
        ] {
            assert!(advertisement(source).is_none(), "{source}");
        }
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;
    fn control(source: &str) -> Option<OriginalPackageIndexControl> {
        let config = tcl_lexer::LexerConfig::default();
        let survey = OriginalPackageIndexSourceSurvey::capture(
            source,
            &CommandRegistry::build_default(),
            config,
        );
        let plan = tcl_lexer::native_script_words_in(
            survey.source_image().clone(),
            tcl_lexer::Span::new(0, source.len() as u32),
            config,
        )
        .ok()?;
        survey.control(plan.commands.last()?)
    }
    #[test]
    fn source_control_retains_real_lookup_and_selected_completion() {
        assert!(matches!(
            control("return").unwrap().completion(),
            tcl_registry::registry::InvocationCompletion::ReturnsResult(_)
        ));
        assert!(control("proc return args {}; return").is_none());
        assert!(control("mystery; return").is_none());
        assert!(control("rename return gone; return").is_none());
    }
    #[test]
    fn retained_control_keeps_actual_body_and_parent_model_lineage() {
        let source = "if 1 \"ret\\u0075rn\"";
        let config = tcl_lexer::LexerConfig::default();
        let survey = OriginalPackageIndexSourceSurvey::capture(
            source,
            &CommandRegistry::build_default(),
            config,
        );
        let plan = tcl_lexer::native_script_words_in(
            survey.source_image().clone(),
            tcl_lexer::Span::new(0, source.len() as u32),
            config,
        )
        .unwrap();
        let body = survey.retained_body(&plan.commands[0], 1).unwrap();
        let start = body.script().base();
        let end = start + u32::try_from(body.script().text.len()).unwrap();
        let child = tcl_lexer::native_script_words_in(
            body.survey().source_image().clone(),
            tcl_lexer::Span::new(start, end),
            config,
        )
        .unwrap();
        let control = body.survey().control(&child.commands[0]).unwrap();
        assert_eq!(control._ancestors.len(), 1);
        assert!(matches!(
            control.completion(),
            tcl_registry::registry::InvocationCompletion::ReturnsResult(_)
        ));
    }
}
