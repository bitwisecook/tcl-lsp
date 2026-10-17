// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional package-index paths from genuine original source schemas.

use super::{AfterIf, Condition, Conditions, Guard, OriginalScript, Reached};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::source_structure::{
    OriginalPackageIndexAdvertisement, OriginalPackageIndexControl,
    OriginalPackageIndexSourceSurvey, OriginalRegistrySource, OriginalRegistryWords,
};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

pub(super) fn scan(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(Reached<'_>),
) -> Option<()> {
    scan_disposition(source, analysis, registry, config, visit).map(|_| ())
}

pub(super) fn scan_disposition(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(Reached<'_>),
) -> Option<super::OriginalSourceSurveyDisposition> {
    // naming.package.original-reachability-source-controls
    // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
    let image = SourceImage::document(source);
    if analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&image, config)
        || analysis.resolved_registry()?.snapshot().semantic_key()
            != registry.snapshot().semantic_key()
    {
        return None;
    }
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let end = u32::try_from(source.len()).ok()?;
    let mut walker = Walker {
        source,
        analysis: Some(analysis),
        survey: None,
        image: image.clone(),
        registry,
        context: &context,
        config,
        visit,
        complete: true,
        syntax_only: false,
    };
    let after = walker.script(Span::new(0, end), &Conditions::default(), 0);
    Some(match (walker.complete, after) {
        (true, Some(conditions)) => super::OriginalSourceSurveyDisposition::Complete(conditions),
        (true, None) => super::OriginalSourceSurveyDisposition::Stopped,
        (false, after) => super::OriginalSourceSurveyDisposition::Incomplete(
            after.unwrap_or_default().with(Condition::Undecidable),
        ),
    })
}

pub(super) fn scan_survey(
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(Reached<'_>),
) -> Option<super::OriginalSourceSurveyDisposition> {
    let survey = OriginalPackageIndexSourceSurvey::capture(source, registry, config);
    let context = survey.editing_input().context_registry();
    let end = u32::try_from(source.len()).ok()?;
    let mut walker = Walker {
        source,
        analysis: None,
        survey: Some(&survey),
        image: survey.source_image().clone(),
        registry,
        context: &context,
        config,
        visit,
        complete: true,
        syntax_only: false,
    };
    let after = walker.script(Span::new(0, end), &Conditions::default(), 0);
    Some(match (walker.complete, after) {
        (true, Some(conditions)) => super::OriginalSourceSurveyDisposition::Complete(conditions),
        (true, None) => super::OriginalSourceSurveyDisposition::Stopped,
        (false, after) => super::OriginalSourceSurveyDisposition::Incomplete(
            after.unwrap_or_default().with(Condition::Undecidable),
        ),
    })
}

struct Control {
    words: OriginalRegistryWords,
    traits: Traits,
    chain: bool,
    registration: bool,
    stops: bool,
    closed: bool,
    original: tcl_lexer::NativeScriptCommandWords,
    source_control: Option<OriginalPackageIndexControl>,
    source_clauses: Option<Vec<(Option<usize>, usize)>>,
    roles_complete: bool,
    advertisement: Option<std::sync::Arc<OriginalPackageIndexAdvertisement>>,
}
impl Control {
    fn path_admitted(&self) -> bool {
        self.closed || self.source_control.is_some()
    }
}

struct Walker<'a, 'v> {
    source: &'a str,
    analysis: Option<&'a AnalysisResult>,
    survey: Option<&'a OriginalPackageIndexSourceSurvey>,
    image: SourceImage,
    registry: &'a CommandRegistry,
    context: &'a tcl_registry::model::ContextRegistry,
    config: LexerConfig,
    visit: &'v mut dyn FnMut(Reached<'_>),
    // Coverage is sticky and independent of continuation/guard conditions.
    complete: bool,
    syntax_only: bool,
}

impl Walker<'_, '_> {
    fn control(
        &self,
        segment: &tcl_compiler::segmenter::SegmentedCommand,
        original: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<Control> {
        let words = if let Some(survey) = self.survey {
            survey.words(segment)?
        } else {
            tcl_compiler::registry_invocation::source_structure::source_registry_words(
                self.source,
                self.analysis?,
                segment,
            )?
        };
        let source_control = (!self.syntax_only)
            .then_some(self.survey)
            .flatten()
            .and_then(|survey| survey.control(original));
        let advertisement = self
            .survey
            .and_then(|survey| survey.advertisement(original))
            .map(std::sync::Arc::new);
        let closed = matches!(words.source(), OriginalRegistrySource::Selected)
            && words.operands_preserve_source_lookup();
        let (traits, chain, registration, stops, source_clauses, roles_complete) = words.with_source_schema(self.context, |schema| {
            let traits = schema.semantics.traits;
            let source_clauses = if self.survey.is_some() && traits.contains(Traits::HAS_BOOLEAN_COND)
                && schema.semantics.clause_grammar.is_some_and(|grammar| grammar.selection == tcl_registry::ClauseSelection::FirstMatch) {
                schema.clause_plan().filter(|plan| plan.defect.is_none()).and_then(|plan| {
                    plan.clauses.iter().map(|clause| {
                        if clause.falls_through_to.is_some() { return None; }
                        let bodies = clause.operands.iter().filter(|(_, slot)| slot.role == ArgRole::Body).map(|(argument, _)| *argument).collect::<Vec<_>>();
                        let expressions = clause.operands.iter().filter(|(_, slot)| slot.role == ArgRole::Expr).map(|(argument, _)| *argument).collect::<Vec<_>>();
                        let [body] = bodies.as_slice() else { return None; };
                        match expressions.as_slice() { [] if clause.is_default => Some((None,*body)), [expr] => Some((Some(*expr),*body)), _ => None }
                    }).collect::<Option<Vec<_>>>()
                })
            } else { None };
            let chain = if self.survey.is_some() { source_clauses.is_some() } else {
                traits.contains(Traits::HAS_BOOLEAN_COND)
                    && (schema.semantics.arg_role_resolver.is_some() || schema.semantics.arg_role_layout_resolver.is_some())
            };
            let roles_complete = schema.authored_source_argument_roles().1;
            let registration = schema.semantics.analyser_hook
                == Some(tcl_registry::hooks::AnalyserHookId::PackageIfneeded)
                && schema.words.arguments().exact_argv_len().is_some_and(|count|
                    count == schema.semantics.argument_offset.saturating_add(3))
                && words.origins().iter().skip(1).enumerate().all(|(argument, origin)| {
                    matches!(origin, tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)
                        if *written == argument + 1)
                });
            let stops = if self.survey.is_some() {
                source_control.as_ref().is_some_and(|control| traits.contains(Traits::TERMINATES_BLOCK)
                    && matches!(control.completion(), tcl_registry::registry::InvocationCompletion::ReturnsResult(_) | tcl_registry::registry::InvocationCompletion::Terminates))
            } else { closed && traits.contains(Traits::TERMINATES_BLOCK)
                && self.registry.invocation_completion_route(
                    schema.canonical_command,
                    schema.words.arguments(),
                    Some(self.context.context().authoring_query()),
                ).is_some_and(|route| !route.normal_possible()) };
            (traits, chain, registration, stops, source_clauses, roles_complete)
        })?;
        Some(Control {
            words,
            traits,
            chain,
            registration,
            stops,
            closed,
            original: original.clone(),
            source_control,
            source_clauses,
            roles_complete,
            advertisement,
        })
    }

    fn script(&mut self, region: Span, reached: &Conditions, depth: u32) -> Option<Conditions> {
        if super::MAX_GUARD_NESTING_DEPTH.exceeded(depth) {
            self.complete = false;
            return Some(reached.with(Condition::Undecidable));
        }
        let Some(text) = self.source.get(region.as_range()) else {
            self.complete = false;
            return Some(reached.with(Condition::Undecidable));
        };
        let original = OriginalScript {
            image: self.image.clone(),
            span: region,
        };
        let Ok(plan) =
            tcl_lexer::native_script_words_in(original.image.clone(), region, self.config)
        else {
            self.complete = false;
            return Some(reached.with(Condition::Undecidable));
        };
        if plan.fatal_tail.is_some() {
            self.complete = false;
            return Some(reached.with(Condition::Undecidable));
        }
        let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            text,
            region.start(),
            self.config.at_depth(depth),
        );
        let commands = super::walk_command_words_with_config(text, self.config.at_depth(depth));
        // Coverage needs the whole original vector inventory, not merely a
        // successful join for the subset the compatibility walker reports.
        if commands.len() != plan.commands.len()
            || commands
                .iter()
                .zip(&plan.commands)
                .any(|(words, expected)| {
                    original.words(words, &plan).as_deref() != Some(expected.words.as_slice())
                })
        {
            self.complete = false;
        }
        let mut reached = reached.clone();
        for words in &commands {
            let Some(native) = original.words(words, &plan) else {
                self.complete = false;
                reached = reached.with(Condition::Undecidable);
                continue;
            };
            let Some(head) = native.first() else {
                continue;
            };
            let Some(control) = segments
                .iter()
                .find(|segment| segment.span.start() == head.span().start())
                .and_then(|segment| {
                    let command = plan
                        .commands
                        .iter()
                        .find(|command| command.words == native)?;
                    self.control(segment, command)
                })
            else {
                self.complete = false;
                reached = reached.with(Condition::Undecidable);
                continue;
            };
            // The existing walker does not traverse operand substitutions. The
            // shared full-depth arena includes commands inside variable indexes.
            // Record omitted script/Expr/error phases BEFORE callbacks or stops.
            if native.iter().any(|word| {
                word.executable_parts().all_parts().any(|part| {
                    matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Command { .. }
                            | tcl_lexer::ExecutablePart::Expression { .. }
                            | tcl_lexer::ExecutablePart::ParseError(_)
                    )
                })
            }) {
                self.complete = false;
            }
            let (bodies, bodies_complete) = self.bodies(&control.words);
            if !control.path_admitted()
                || !bodies_complete
                || (self.survey.is_some() && !control.roles_complete)
            {
                self.complete = false;
            }
            match control.words.roles() {
                Some(roles) => {
                    for &(argument, role) in roles {
                        if role == ArgRole::Expr
                            && !matches!(self.guard(&control, argument), Guard::Constant(_))
                        {
                            self.complete = false;
                        }
                    }
                }
                None => self.complete = false,
            }
            if control.registration {
                let conditions = if control.path_admitted() || control.advertisement.is_some() {
                    reached.clone()
                } else {
                    reached.with(Condition::Undecidable)
                };
                let conditions = control.advertisement.as_ref().map_or_else(
                    || conditions.clone(),
                    |receipt| conditions.with_advertisement(std::sync::Arc::clone(receipt)),
                );
                (self.visit)(Reached {
                    text,
                    words,
                    conditions,
                    original_words: Some(native),
                });
                continue;
            }
            if control.stops {
                return None;
            }
            if control.chain {
                match self.chain(&control, &reached, depth) {
                    AfterIf::Stops => return None,
                    AfterIf::Continues(after) => {
                        reached = after;
                        continue;
                    }
                    AfterIf::NotAChain => {}
                }
            }
            let mut might_stop = false;
            for body in bodies {
                let body_reached = reached.with(Condition::Undecidable);
                let after = self.walk_body(&control, body, &body_reached, depth + 1);
                might_stop |= after.is_none()
                    || (self.survey.is_some()
                        && after
                            .as_ref()
                            .is_some_and(|after| additional_unknown(after, &body_reached)));
            }
            if !control.path_admitted()
                || (control.traits.contains(Traits::CONTROL_FLOW) && might_stop)
            {
                reached = reached.with(Condition::Undecidable);
            }
        }
        Some(reached)
    }

    fn walk_body(
        &mut self,
        control: &Control,
        region: Span,
        reached: &Conditions,
        depth: u32,
    ) -> Option<Conditions> {
        if let Some(survey) = self.survey {
            let argument = control.words.roles().and_then(|roles| {
                roles.iter().find_map(|(argument, role)| {
                    (*role == ArgRole::Body && self.body(&control.words, *argument) == Some(region))
                        .then_some(*argument)
                })
            });
            let body =
                argument.and_then(|argument| survey.retained_body(&control.original, argument));
            let Some(body) = body else {
                // Source Body catalogue, not an entered script. It retains the
                // original parent graph and records uncertainty. Only unchanged
                // braced geometry can be visited here; no control/model grant.
                let source_literal = control
                    .words
                    .source_script_bodies(self.context)
                    .into_iter()
                    .chain(control.words.original_lambda_body_geometry(self.context))
                    .any(|body| {
                        body.matches_source(&self.image, self.config)
                            && body.content_span() == region
                    });
                self.complete = false;
                if !source_literal {
                    return Some(reached.with(Condition::Undecidable));
                }
                let previous = self.syntax_only;
                self.syntax_only = true;
                let after = self.script(region, reached, depth);
                self.syntax_only = previous;
                return after;
            };
            let child = body.survey();
            let Some(source) = child.source_image().try_text().ok() else {
                self.complete = false;
                return Some(reached.with(Condition::Undecidable));
            };
            let start = body.script().base();
            let Some(end) = u32::try_from(body.script().text.len())
                .ok()
                .and_then(|len| start.checked_add(len))
            else {
                self.complete = false;
                return Some(reached.with(Condition::Undecidable));
            };
            let mut walker = Walker {
                source,
                analysis: None,
                survey: Some(child),
                image: child.source_image().clone(),
                registry: self.registry,
                context: self.context,
                config: self.config,
                visit: self.visit,
                complete: true,
                syntax_only: self.syntax_only,
            };
            let after = walker.script(Span::new(start, end), reached, depth);
            self.complete &= walker.complete;
            after
        } else {
            self.script(region, reached, depth)
        }
    }

    fn body(&self, words: &OriginalRegistryWords, argument: usize) -> Option<Span> {
        let operand = words.operands().get(argument)?.as_ref()?;
        let word = operand.word()?;
        let content = word.content_span().ok()?;
        if self.survey.is_some() {
            return (word.image() == &self.image
                && word.config() == self.config
                && !word.group().expand
                && tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some())
            .then_some(content);
        }
        (word.image() == &self.image
            && word.config() == self.config
            && operand.input()?.bytes() == self.source.get(content.as_range())?.as_bytes())
        .then_some(content)
    }

    fn bodies(&self, words: &OriginalRegistryWords) -> (Vec<Span>, bool) {
        let mut bodies = Vec::new();
        let Some(roles) = words.roles() else {
            return (bodies, false);
        };
        let mut complete = true;
        for &(argument, role) in roles {
            if role == ArgRole::Body {
                match self.body(words, argument) {
                    Some(body) => bodies.push(body),
                    None => complete = false,
                }
            } else if role == ArgRole::LambdaLiteral {
                if self.survey.is_some() {
                    match words.original_lambda_body_geometry(self.context) {
                        Some(body) if body.matches_source(&self.image, self.config) => {
                            bodies.push(body.content_span())
                        }
                        _ => complete = false,
                    }
                    continue;
                }
                let body = words
                    .operands()
                    .get(argument)
                    .and_then(Option::as_ref)
                    .and_then(|operand| operand.word())
                    .and_then(|word| {
                        (word.image() == &self.image && word.config() == self.config)
                            .then_some(word)
                    })
                    .and_then(tcl_compiler::lambda_literal::split_original_lambda_literal)
                    .and_then(|elements| elements.braced_body());
                match body {
                    Some(body) => bodies.push(body),
                    None => complete = false,
                }
            }
        }
        (bodies, complete)
    }

    fn chain(&mut self, control: &Control, reached: &Conditions, depth: u32) -> AfterIf {
        let Some(roles) = control.words.roles() else {
            self.complete = false;
            return AfterIf::NotAChain;
        };
        let bodies: Vec<_> = if let Some(clauses) = &control.source_clauses {
            clauses.iter().map(|(_, body)| *body).collect()
        } else {
            roles
                .iter()
                .filter(|(_, role)| *role == ArgRole::Body)
                .map(|(argument, _)| *argument)
                .collect()
        };
        if bodies.is_empty() {
            self.complete = false;
            return AfterIf::NotAChain;
        }
        let body_count = bodies.len();
        let mut earlier_false = Conditions::default();
        let mut terminating = Vec::new();
        let mut definitely_stopped = 0usize;
        let mut has_else = false;
        let mut definitely_taken_terminates = false;
        let mut previous = None;
        let mut missing_body = false;
        for body in bodies {
            let expression = if let Some(clauses) = &control.source_clauses {
                clauses
                    .iter()
                    .find(|(_, selected_body)| *selected_body == body)
                    .and_then(|(expression, _)| *expression)
            } else {
                roles
                    .iter()
                    .filter(|(argument, role)| {
                        *role == ArgRole::Expr
                            && *argument < body
                            && previous.is_none_or(|previous| *argument > previous)
                    })
                    .map(|(argument, _)| *argument)
                    .max()
            };
            previous = Some(body);
            let mut branch = earlier_false.clone();
            if let Some(expression) = expression {
                let guard = if control.path_admitted() {
                    self.guard(control, expression)
                } else {
                    Guard::Unknown
                };
                if matches!(guard, Guard::Unknown) {
                    self.complete = false;
                }
                if let Some(condition) = guard.condition(true) {
                    branch = branch.with(condition);
                }
                if let Some(condition) = guard.condition(false) {
                    earlier_false = earlier_false.with(condition);
                }
            } else {
                has_else = true;
                if !control.path_admitted() {
                    branch = branch.with(Condition::Undecidable);
                }
            }
            let mut branch_reached = reached.clone();
            for condition in &branch.0 {
                branch_reached = branch_reached.with(condition.clone());
            }
            let Some(region) = self.body(&control.words, body) else {
                self.complete = false;
                missing_body = true;
                // One missing branch must not suppress discoveries in known siblings.
                continue;
            };
            let after = self.walk_body(control, region, &branch_reached, depth + 1);
            if after.is_none() {
                definitely_stopped += 1;
                definitely_taken_terminates |= control.path_admitted() && branch.is_unconditional();
                terminating.push(match branch.0.as_slice() {
                    [only] => Some(only.clone()),
                    _ => None,
                });
            } else if self.survey.is_some()
                && !branch
                    .0
                    .iter()
                    .any(|condition| matches!(condition, Condition::Impossible))
                && after
                    .as_ref()
                    .is_some_and(|after| additional_unknown(after, &branch_reached))
            {
                terminating.push(Some(Condition::Undecidable));
            }
        }
        if missing_body {
            return AfterIf::Continues(reached.with(Condition::Undecidable));
        }
        if control.path_admitted()
            && (definitely_taken_terminates || (has_else && definitely_stopped == body_count))
        {
            return AfterIf::Stops;
        }
        AfterIf::Continues(match terminating.as_slice() {
            [] => reached.clone(),
            [Some(condition)] if control.path_admitted() => condition
                .negated()
                .map_or_else(|| reached.clone(), |condition| reached.with(condition)),
            _ => reached.with(Condition::Undecidable),
        })
    }

    fn guard(&self, control: &Control, argument: usize) -> Guard {
        let words = &control.words;
        if let Some(survey) = self.survey.filter(|_| !self.syntax_only) {
            if let Some(receipt) = survey.target_guard(&control.original, argument) {
                return Guard::SourceTarget(super::SourceTargetCondition {
                    receipt: std::sync::Arc::new(receipt),
                    expect: true,
                });
            }
        }
        let Some(body) = self.body(words, argument) else {
            return Guard::Unknown;
        };
        let Some(text) = self.source.get(body.as_range()) else {
            return Guard::Unknown;
        };
        match super::constant_guard(text, self.config) {
            guard @ Guard::Constant(_) => guard,
            // Selected package-query syntax alone cannot establish that the
            // current Tcl package provision still equals the running release.
            #[cfg(test)]
            Guard::TclSatisfies { .. } => Guard::Unknown,
            Guard::SourceTarget(_) | Guard::Unknown => Guard::Unknown,
        }
    }
}

fn additional_unknown(after: &Conditions, before: &Conditions) -> bool {
    let count = |conditions: &Conditions| {
        conditions
            .0
            .iter()
            .filter(|condition| matches!(condition, Condition::Undecidable))
            .count()
    };
    count(after) > count(before)
}

#[cfg(test)]
mod tests {
    use super::super::Availability;
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn registrations(source: &str, analysis: &AnalysisResult) -> Vec<(String, Availability)> {
        let mut found = Vec::new();
        scan(
            source,
            analysis,
            analysis.resolved_registry().unwrap(),
            analysis.body_lexer_config.unwrap(),
            &mut |reached| {
                let input = reached.original_words.as_ref().unwrap().get(2).unwrap();
                let policy = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                    .authored_name_policy().unwrap();
                let key = tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    input,
                    tcl_syntax::word_rules::WordValueRules::from_config(&analysis.body_lexer_config.unwrap()),
                    policy,
                ).unwrap();
                found.push((key.display().unwrap().to_owned(), reached.conditions.availability(Some(tcl_dialect::TclVersion::V8_6))));
            },
        ).unwrap();
        found
    }

    #[test]
    fn original_package_paths_use_current_control_roles_and_completion_options() {
        // naming.package.original-reachability-source-controls
        // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
        for (source, expected) in [
            (
                "package ifneeded p 1.0 {source p.tcl}",
                vec![("p".to_owned(), Availability::Available)],
            ),
            (
                "if {1} {package ifneeded p 1.0 {source p.tcl}}",
                vec![("p".to_owned(), Availability::Available)],
            ),
            (
                "if {0} {package ifneeded p 1.0 {source p.tcl}}",
                vec![("p".to_owned(), Availability::Unavailable)],
            ),
            ("return; package ifneeded p 1.0 {source p.tcl}", vec![]),
            (
                "return -level 0 ignored; package ifneeded p 1.0 {source p.tcl}",
                vec![("p".to_owned(), Availability::Available)],
            ),
            (
                "proc if args {return ignored}; if {1} {package ifneeded p 1.0 {source p.tcl}}",
                vec![],
            ),
            (
                "proc package args {return ignored}; package ifneeded p 1.0 {source p.tcl}",
                vec![],
            ),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.all_procs.clear();
            analysis.command_invocations.clear();
            assert_eq!(registrations(source, &analysis), expected, "{source}");
        }
        let source =
            "proc return args {return ignored}; return; package ifneeded p 1.0 {source p.tcl}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            registrations(source, &analysis).len(),
            1,
            "a custom return must not prune by a builtin trait"
        );
    }

    #[test]
    fn original_package_paths_preserve_current_loader_axes_and_version_uncertainty() {
        // naming.package.original-reachability-source-controls
        // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
        let source = "package ifneeded p 1.0 {source p.tcl}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let registry = analysis.resolved_registry().unwrap();
        let config = analysis.body_lexer_config.unwrap();
        let mut visits = 0;
        let mut visit = |_: Reached<'_>| visits += 1;
        assert!(
            scan(
                &format!("{source} "),
                &analysis,
                registry,
                config,
                &mut visit
            )
            .is_none()
        );
        assert!(
            scan(
                source,
                &analysis,
                crate::registry_for_dialect("jim"),
                config,
                &mut visit
            )
            .is_none()
        );
        assert!(
            scan(
                source,
                &analysis,
                registry,
                LexerConfig {
                    strict_quoting: !config.strict_quoting,
                    ..config
                },
                &mut visit
            )
            .is_none()
        );
        assert_eq!(visits, 0);
        let source = "if {[package vsatisfies [package provide Tcl] 9-]} {package ifneeded p 1.0 {source p.tcl}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let found = registrations(source, &analysis);
        assert_eq!(found, [("p".to_owned(), Availability::Conditional)]);
    }
}

#[cfg(test)]
mod original_package_region_purpose_tests {
    use super::*;

    #[test]
    fn original_package_body_words_keep_parent_image_and_nested_bom_geometry() {
        // naming.package.original-reachability-source-controls
        // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
        // This joins original lexical vectors; it does not close an unknown loader call.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        for source in [
            "if 1 {\u{feff}package ifneeded p 1.0 {source p.tcl}}",
            "if 1 {package ifneeded p 1.0 {source p.tcl}}",
        ] {
            let mut analyser = tcl_compiler::analyser::Analyser::new();
            let analysis = analyser.analyse(source, "tcl9.0");
            assert!(analysis.matches_original_source_image(&SourceImage::document(source), config));
            let root = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let region = root.commands[0].words[2].content_span().unwrap();
            let original = OriginalScript {
                image: SourceImage::document(source),
                span: region,
            };
            let plan =
                tcl_lexer::native_script_words_in(original.image.clone(), region, config).unwrap();
            let text = source.get(region.as_range()).unwrap();
            let commands = super::super::walk_command_words_with_config(text, config.at_depth(1));
            let words = original
                .words(&commands[0], &plan)
                .expect("complete original body vector");
            assert_eq!(words[0].config(), config.normalized());
            assert_eq!(words[0].span().start(), region.start());
            assert_eq!(words[0].image().bytes(), source.as_bytes());
            assert_eq!(
                text.get(commands[0][0][0].span.as_range()),
                Some(if text.starts_with('\u{feff}') {
                    "\u{feff}package"
                } else {
                    "package"
                })
            );
            let wrong = super::super::walk_command_words_with_config(text, config);
            if text.starts_with('\u{feff}') {
                assert!(original.words(&wrong[0], &plan).is_none());
            }
        }
        let body = "puts ${a{b}; package ifneeded p 1.0 {source p.tcl}";
        let first_close = LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..config
        };
        assert_eq!(
            super::super::walk_command_words_with_config(body, first_close.at_depth(1)).len(),
            2
        );
        assert!(super::super::walk_command_words_with_config(body, config.at_depth(1)).is_empty());
    }
}

#[cfg(test)]
mod coverage_controls {
    use super::super::OriginalSourceSurveyDisposition;
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn disposition(source: &str) -> OriginalSourceSurveyDisposition {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        scan_disposition(
            source,
            &analysis,
            analysis.resolved_registry().unwrap(),
            analysis.body_lexer_config.unwrap(),
            &mut |_| {},
        )
        .unwrap()
    }

    #[test]
    fn original_coverage_keeps_complete_and_stopped_distinct() {
        assert!(matches!(
            disposition("set value 1"),
            OriginalSourceSurveyDisposition::Complete(_)
        ));
        assert_eq!(
            disposition("return"),
            OriginalSourceSurveyDisposition::Stopped
        );
    }

    #[test]
    fn original_coverage_checks_omitted_operands_before_registration_and_stop() {
        for source in [
            "package ifneeded p 1.0 [list source p.tcl]",
            "return [list ignored]",
            "return $array([list index])",
        ] {
            assert!(
                matches!(
                    disposition(source),
                    OriginalSourceSurveyDisposition::Incomplete(_)
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn original_coverage_keeps_unknown_expression_and_missing_body_incomplete() {
        for source in [
            "if {$condition} {set x 1}",
            "if {1} $body",
            "expr {[list value]}",
            "if",
        ] {
            assert!(
                matches!(
                    disposition(source),
                    OriginalSourceSurveyDisposition::Incomplete(_)
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn original_coverage_keeps_malformed_source_and_missing_schema_incomplete() {
        for source in ["list {unterminated", "unknown_command"] {
            assert!(
                matches!(
                    disposition(source),
                    OriginalSourceSurveyDisposition::Incomplete(_)
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn original_coverage_preserves_known_registration_callbacks() {
        let source = "package ifneeded p 1.0 {source p.tcl}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let mut callbacks = 0;
        let result = scan_disposition(
            source,
            &analysis,
            analysis.resolved_registry().unwrap(),
            analysis.body_lexer_config.unwrap(),
            &mut |reached| {
                callbacks += 1;
                assert!(reached.original_words.is_some());
            },
        );
        assert!(result.is_some());
        assert_eq!(callbacks, 1);
    }

    #[test]
    fn original_coverage_rejects_foreign_source_without_callbacks() {
        let source = "package ifneeded p 1.0 {source p.tcl}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let mut callbacks = 0;
        assert!(
            scan_disposition(
                "package ifneeded q 1.0 {source q.tcl}",
                &analysis,
                analysis.resolved_registry().unwrap(),
                analysis.body_lexer_config.unwrap(),
                &mut |_| callbacks += 1
            )
            .is_none()
        );
        assert_eq!(callbacks, 0);
    }
}

#[cfg(test)]
mod complete_vector_control {
    use super::*;
    #[test]
    fn complete_source_vectors_keep_quoted_and_braced_data_in_one_command() {
        let source = "list {a;b} \"quoted value\"; set output 2";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(matches!(
            scan_disposition(
                source,
                &analysis,
                analysis.resolved_registry().unwrap(),
                analysis.body_lexer_config.unwrap(),
                &mut |_| {}
            ),
            Some(super::super::OriginalSourceSurveyDisposition::Complete(_))
        ));
    }
}
