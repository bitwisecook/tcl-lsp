// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fold requests selected for an owned application evaluator.
//!
//! Selection runs the ordinary retained-source gates of the O129 fold but not
//! the selected outer implementation. The result is request data only: no
//! rewrite, execution, cache entry or Native permission follows from it, and
//! its holder decides which evaluator, if any, answers it.

use tcl_registry::CommandRegistry;
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;

use crate::compilation_unit::CompilationUnit;
use crate::const_subst::{ConstSubstCtx, SelectedConstFold};
use crate::ir::WordExpr;

/// Maximum accepted UTF-8 source bytes for request selection.
pub const MAX_APPLICATION_SOURCE_BYTES: usize = 4096;

/// One whole-word command substitution of a top-level command, with the fold
/// request the ordinary optimiser would have evaluated for it.
#[derive(Debug, Clone)]
pub struct SelectedApplicationFold {
    span: std::ops::Range<usize>,
    request: SelectedConstFold,
    release: tcl_dialect::TclVersion,
}

impl SelectedApplicationFold {
    /// Byte range of the whole `[…]` in the selected source.
    #[must_use]
    pub fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    /// The checked request; it has not been evaluated.
    #[must_use]
    pub fn request(&self) -> &SelectedConstFold {
        &self.request
    }

    /// Whether `value` may stand for the substitution as the literal it is
    /// spelt, under the same release rule the ordinary fold applies.
    #[must_use]
    pub fn admits_answer(&self, value: &str) -> bool {
        !crate::const_subst::folded_literal_reads_apart(value, self.release)
    }
}

/// Select the fold request of every whole-word command substitution in the
/// top-level commands of `source`. Source declaring a procedure or other
/// lowered body is refused, and a command whose
/// retained substitution inventory is unavailable selects nothing.
///
/// # Errors
/// Refuses oversized or procedure-bearing source, a profile without an actual
/// Tcl execution point, and a module without retained source metadata.
pub fn select_application_folds(
    source: &str,
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<Vec<SelectedApplicationFold>, ClosedSourceRefusal> {
    if source.len() > MAX_APPLICATION_SOURCE_BYTES {
        return Err(ClosedSourceRefusal::Budget);
    }
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    if dialect.execution_point().is_none()
        || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile);
    let module = &cu.ir_module;
    if !module.procedures.is_empty()
        || module.body_lowerings().next().is_some()
        || module.source_metadata_input.is_none()
    {
        return Err(ClosedSourceRefusal::Source);
    }
    let retained = crate::registry_invocation::retained_module_metadata_context(registry, module)
        .ok_or(ClosedSourceRefusal::Source)?;
    let metadata: crate::registry_invocation::InvocationMetadataContext<'_> = (&*retained).into();
    let mut selected = Vec::new();
    for statement in &module.top_level.statements {
        // The ordinary O129 visitor's sites only.
        if !matches!(
            statement,
            crate::ir::Statement::Call { .. } | crate::ir::Statement::AssignValue { .. }
        ) {
            continue;
        }
        let Some(tokens) = statement.tokens() else {
            continue;
        };
        let Some(calls) =
            crate::optimiser::propagation::retained_substitution_calls(tokens, module, registry)
        else {
            continue;
        };
        // Whole-word substitutions after the command word; never a head,
        // an expansion or a template part.
        for word in tokens.words().iter().skip(1) {
            let WordExpr::CommandSubstitution { source: site, .. } = word else {
                continue;
            };
            let Some(call) =
                crate::optimiser::propagation::retained_call_at(&calls, site.span.start())
            else {
                continue;
            };
            let context = ConstSubstCtx {
                registry,
                resolution_namespace: "::",
                namespace_context: call
                    .tokens
                    .as_ref()
                    .and_then(crate::registry_invocation::compiled_namespace_context),
                version: None,
                defining_class: None,
                trusts: &|_| false,
                lookup_var: &|_| None,
            };
            if let Some(request) =
                context.select_retained_call_with_metadata_context(call, &calls, metadata)
            {
                selected.push(SelectedApplicationFold {
                    span: call.span.as_range(),
                    request,
                    release: profile.vm_runtime_version,
                });
            }
        }
    }
    Ok(selected)
}

/// Upper bound on top-level commands in an application candidate.
pub const MAX_APPLICATION_COMMANDS: usize = 64;

/// Which admitted text a rewrite proposal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationText {
    /// The original, admitted for correspondence only; it is never executed.
    Original,
    /// The planned candidate.
    Candidate,
}

/// The original source, its candidate with each answered substitution
/// replaced by its rendered literal, and the candidate's complete command and
/// scalar inventory. The answers' authority is the fold that selected them;
/// this plan only proves the rewrite's byte correspondence and closes the
/// candidate's effects.
#[derive(Debug, Clone)]
pub struct ApplicationRewritePlan {
    source: String,
    candidate: String,
    profile: &'static tcl_dialect::DialectProfile,
    registry: tcl_registry::RegistrySemanticKey,
    commands: Vec<String>,
    scalars: Vec<String>,
}

impl ApplicationRewritePlan {
    /// Replace each selected substitution by its rendered answer and close
    /// the candidate.
    ///
    /// # Errors
    /// Refuses an empty or unrenderable answer set, an answer that reads apart
    /// from its literal, overlapping sites, command-span drift, and any
    /// candidate command outside the admitted inventory.
    pub fn new(
        source: &str,
        registry: &CommandRegistry,
        profile: &'static tcl_dialect::DialectProfile,
        answers: &[(SelectedApplicationFold, String)],
    ) -> Result<Self, ClosedSourceRefusal> {
        if answers.is_empty() {
            return Err(ClosedSourceRefusal::Source);
        }
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let mut edits = Vec::with_capacity(answers.len());
        for (fold, value) in answers {
            let span = fold.span();
            let site = source
                .get(span.clone())
                .ok_or(ClosedSourceRefusal::Source)?;
            if !fold.admits_answer(value)
                || crate::const_subst::folded_literal_reads_apart(
                    value,
                    tcl_dialect::TclVersion::V9_1,
                )
                || !site.starts_with('[')
                || !site.ends_with(']')
            {
                return Err(ClosedSourceRefusal::Source);
            }
            let word = tcl_syntax::word_rules::lexical_unicode_source_word(value, config)
                .filter(|word| {
                    tcl_syntax::word_rules::lexical_unicode_source_word_matches(word, value, config)
                })
                .ok_or(ClosedSourceRefusal::Source)?;
            edits.push((span, word));
        }
        edits.sort_by_key(|(span, _)| span.start);
        if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
            return Err(ClosedSourceRefusal::Source);
        }
        let mut candidate = source.to_owned();
        for (span, word) in edits.iter().rev() {
            candidate.replace_range(span.clone(), word);
        }
        corresponding_commands(source, &candidate, &edits, config)?;
        let (commands, scalars) = candidate_inventory(&candidate, registry, profile)?;
        Ok(Self {
            source: source.to_owned(),
            candidate,
            profile,
            registry: registry.snapshot().semantic_key(),
            commands,
            scalars,
        })
    }

    /// The original source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The candidate source.
    #[must_use]
    pub fn candidate(&self) -> &str {
        &self.candidate
    }

    /// Canonical registry commands the candidate invokes.
    #[must_use]
    pub fn command_names(&self) -> &[String] {
        &self.commands
    }

    /// Plain scalars the candidate writes; each must start absent.
    #[must_use]
    pub fn scalar_names(&self) -> &[String] {
        &self.scalars
    }

    /// The registry semantic key the plan was built under.
    #[must_use]
    pub fn registry_key(&self) -> &tcl_registry::RegistrySemanticKey {
        &self.registry
    }

    /// Classify `text`: the original (correspondence only) or the candidate,
    /// whose inventory is recomputed and must be unchanged.
    ///
    /// # Errors
    /// `Currency` for a changed registry, `Source` for any other text or a
    /// changed candidate inventory.
    pub fn accepts(
        &self,
        text: &str,
        registry: &CommandRegistry,
    ) -> Result<ApplicationText, ClosedSourceRefusal> {
        if registry.snapshot().semantic_key() != self.registry {
            return Err(ClosedSourceRefusal::Currency);
        }
        if text == self.source {
            return Ok(ApplicationText::Original);
        }
        if text != self.candidate {
            return Err(ClosedSourceRefusal::Source);
        }
        let (commands, scalars) = candidate_inventory(text, registry, self.profile)?;
        if commands != self.commands || scalars != self.scalars {
            return Err(ClosedSourceRefusal::Source);
        }
        Ok(ApplicationText::Candidate)
    }
}

/// Every original command maps to one candidate command at its offset-adjusted
/// span; only the replaced words differ.
fn corresponding_commands(
    source: &str,
    candidate: &str,
    edits: &[(std::ops::Range<usize>, String)],
    config: tcl_lexer::LexerConfig,
) -> Result<(), ClosedSourceRefusal> {
    let original = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let rewritten = crate::segmenter::segment_commands_with_offset_and_config(candidate, 0, config);
    if original.len() != rewritten.len() {
        return Err(ClosedSourceRefusal::Source);
    }
    let shift = |offset: usize| -> Option<usize> {
        let mut adjusted = offset;
        for (span, word) in edits {
            if span.end <= offset {
                adjusted = adjusted.checked_add(word.len())?.checked_sub(span.len())?;
            } else if span.start < offset {
                return None;
            }
        }
        Some(adjusted)
    };
    for (before, after) in original.iter().zip(&rewritten) {
        let range = before.span.as_range();
        let start = shift(range.start).ok_or(ClosedSourceRefusal::Source)?;
        let end = shift(range.end).ok_or(ClosedSourceRefusal::Source)?;
        if before.is_partial || after.is_partial || after.span.as_range() != (start..end) {
            return Err(ClosedSourceRefusal::Source);
        }
    }
    Ok(())
}

/// Commands and written scalars of a candidate, admitted from the registry's
/// semantic operation: a plain scalar `set` of a literal, or a channel write
/// whose channel, when named, is the literal standard output. Everything else,
/// including any command that can take a script body or command prefix, is
/// refused.
fn candidate_inventory(
    text: &str,
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<(Vec<String>, Vec<String>), ClosedSourceRefusal> {
    use tcl_registry::hooks::LoweringHookId;
    use tcl_registry::{ArgRole, IntrinsicId, SemanticOperationId};
    if text.len() > MAX_APPLICATION_SOURCE_BYTES {
        return Err(ClosedSourceRefusal::Budget);
    }
    let cu = CompilationUnit::build_for_profile(text, registry, false, profile);
    let module = &cu.ir_module;
    if !module.procedures.is_empty()
        || module.body_lowerings().next().is_some()
        || module.source_metadata_input.is_none()
        || module.top_level.statements.is_empty()
    {
        return Err(ClosedSourceRefusal::Source);
    }
    if module.top_level.statements.len() > MAX_APPLICATION_COMMANDS {
        return Err(ClosedSourceRefusal::Budget);
    }
    let retained = crate::registry_invocation::retained_module_metadata_context(registry, module)
        .ok_or(ClosedSourceRefusal::Source)?;
    let metadata: crate::registry_invocation::InvocationMetadataContext<'_> = (&*retained).into();
    let mut commands = Vec::new();
    let mut scalars: Vec<String> = Vec::new();
    for statement in &module.top_level.statements {
        if !matches!(
            statement,
            crate::ir::Statement::Call { .. }
                | crate::ir::Statement::AssignValue { .. }
                | crate::ir::Statement::AssignConst { .. }
        ) {
            return Err(ClosedSourceRefusal::Source);
        }
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .ok_or(ClosedSourceRefusal::Source)?;
        super::original_target(tokens)?;
        let words = tokens.words();
        let Some(WordExpr::Literal { .. }) = words.first() else {
            return Err(ClosedSourceRefusal::Source);
        };
        let invocation =
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            )
            .ok_or(ClosedSourceRefusal::Source)?;
        let name = invocation.facts.canonical_command.clone();
        let spec = registry
            .get_for_surface(&name, Some(metadata.context().authoring_query()))
            .ok_or(ClosedSourceRefusal::Source)?;
        if CommandRegistry::spec_may_have_arg_role(spec, ArgRole::Body)
            || CommandRegistry::spec_may_have_arg_role(spec, ArgRole::CommandPrefix)
            || !spec.command_prefixes.is_empty()
            || spec.command_prefix_resolver.is_some()
        {
            return Err(ClosedSourceRefusal::Source);
        }
        match invocation.facts.operation {
            SemanticOperationId::StructuredLowering(LoweringHookId::Set) => {
                let [_, WordExpr::Literal { text: target, .. }, value] = words else {
                    return Err(ClosedSourceRefusal::Source);
                };
                if !super::plain_scalar(target) || literal_text(value).is_none() {
                    return Err(ClosedSourceRefusal::Source);
                }
                if !scalars.contains(target) {
                    scalars.push(target.clone());
                }
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite) => {
                let mut arguments = Vec::with_capacity(words.len() - 1);
                for word in &words[1..] {
                    arguments.push(argument_text(word, &scalars, profile)?);
                }
                let spelled = arguments.iter().map(String::as_str).collect::<Vec<_>>();
                for index in registry.arg_indices_for_role(&name, &spelled, ArgRole::Channel) {
                    if !matches!(
                        words.get(index + 1),
                        Some(WordExpr::Literal { text, .. }) if text == "stdout"
                    ) {
                        return Err(ClosedSourceRefusal::Source);
                    }
                }
            }
            _ => return Err(ClosedSourceRefusal::Source),
        }
        commands.push(name);
    }
    commands.sort();
    commands.dedup();
    scalars.sort();
    Ok((commands, scalars))
}

/// A word whose value is fixed text: literal, braced, or a quoted template
/// of text alone.
fn literal_text(word: &WordExpr) -> Option<String> {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => Some(text.clone()),
        WordExpr::Template { parts, .. } => parts
            .iter()
            .map(|part| match part {
                crate::ir::WordPart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// A channel-write argument: fixed text, or a read of a scalar the candidate
/// already wrote.
fn argument_text(
    word: &WordExpr,
    scalars: &[String],
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<String, ClosedSourceRefusal> {
    if let Some(text) = literal_text(word) {
        return Ok(text);
    }
    let WordExpr::Variable { spelling, .. } = word else {
        return Err(ClosedSourceRefusal::Source);
    };
    let tree = crate::expr_parser::parse_expr(spelling, Some(profile.name));
    let name = super::simple_reference(&tree)?;
    if !scalars.iter().any(|held| held == name) {
        return Err(ClosedSourceRefusal::Source);
    }
    Ok(spelling.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .and_then(|environment| environment.catalogue_profile())
            .expect("tcl9.1 catalogue profile")
    }

    #[test]
    fn selection_names_the_request_without_running_it() {
        let profile = profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let source = "set s abc\nputs [string length $s]\n";
        let selected =
            select_application_folds(source, &registry, profile).expect("top-level source selects");
        let [fold] = selected.as_slice() else {
            panic!("one selected fold: {selected:?}");
        };
        assert_eq!(&source[fold.span()], "[string length $s]");
        assert_eq!(fold.request().facts().canonical_command, "string");
        assert_eq!(fold.request().arguments(), Some(vec!["abc"]));
        assert!(fold.admits_answer("3"));
    }

    #[test]
    fn selection_refuses_procedures_and_oversized_source() {
        let profile = profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        assert_eq!(
            select_application_folds("proc p {} {string length abc}\np\n", &registry, profile)
                .unwrap_err(),
            ClosedSourceRefusal::Source
        );
        let long = format!("set s {}\n", "x".repeat(MAX_APPLICATION_SOURCE_BYTES));
        assert_eq!(
            select_application_folds(&long, &registry, profile).unwrap_err(),
            ClosedSourceRefusal::Budget
        );
    }
}
