// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Command validity and arity checks emitted during the command walk.
//!
//! These diagnostics decide whether a command invocation is well-formed
//! against the registry and the active dialect: an unknown subcommand
//! (W001), a command disabled in the dialect (W002), an invalid dialect
//! option (W004) or expression operator (W003), wrong argument counts
//! (the arity diagnostics), a malformed `if` (E004), a missing `--` option
//! terminator before a value that looks like an option (W304), an `unset`
//! whose options consume every argument so nothing is unset (W217), and a stub
//! `proc` that shadows a built-in command or `expr` function (W116, W117).
//! The disabled-command, arity, and W304 emitters buffer their candidates
//! and flush them after the walk.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_core_types::DiagCode;
use tcl_registry::Arity;
use tcl_registry::lifecycle::Lifecycle;

use super::helpers::{has_substitution, is_ident_continue};
use crate::analyser::state::Analyser;
use crate::analyser::types::{PendingUserCallArity, Severity};
use crate::expr_ast::ExprNode;
use tcl_dialect::model::SpecSurface;

/// The tight E003 anchor: the span covering the run of *surplus*
/// positional arguments (from the first argument past the command's `max`
/// up to the last argument), plus where a "remove surplus arguments" fix
/// should start deleting — the end of the last *kept* word, so the fix
/// also removes the separating whitespace.
#[derive(Debug, Clone, Copy)]
pub(in crate::analyser) struct ExcessArgs {
    /// Span of the surplus argument run (diagnostic anchor).
    pub span: tcl_lexer::Span,
    /// Deletion start for the removal fix (end of the preceding word).
    pub delete_from: u32,
}

/// A removal proposal needs a consecutive, wholly written suffix of the
/// guaranteed counted operands. Captures, expansions and interleaved options
/// retain the count claim without acquiring a deletion extent.
pub(super) fn source_excess_arguments(
    original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
    count: &tcl_registry::InvocationArgumentCount,
    max: usize,
) -> Option<ExcessArgs> {
    if count.indeterminate {
        return None;
    }
    let surplus = count.operands.get(max..)?;
    let first = *surplus.first()?;
    let written = original.written_index(first)?;
    for (delta, &argument) in surplus.iter().enumerate() {
        if original.written_index(argument)? != written.checked_add(delta)? {
            return None;
        }
    }
    let last = original.word(*surplus.last()?)?;
    if last.span().end() != original.invocation_span().end() {
        return None;
    }
    let delete_from =
        written
            .checked_sub(1)
            .map_or(Some(original.head().span().end()), |previous| {
                original
                    .written_word(previous)
                    .map(|word| word.span().end())
            })?;
    Some(ExcessArgs {
        span: tcl_lexer::Span::new(original.word(first)?.span().start(), last.span().end()),
        delete_from,
    })
}

pub(super) fn source_arity_verdict(
    original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
    display_name: &str,
    arity: Arity,
    count: &tcl_registry::InvocationArgumentCount,
    synopsis: Option<&str>,
) -> Option<crate::analyser::types::Diagnostic> {
    let diagnostic = arity_verdict(
        display_name,
        arity,
        usize::from(count.minimum),
        count.indeterminate,
        original.invocation_span(),
        source_excess_arguments(original, count, usize::from(arity.max)),
        synopsis,
    )?;
    let subject = original.subject_extent(
        crate::analyser::RegistrySourceDiagnosticKind::Arity,
        diagnostic.span,
    )?;
    Some(diagnostic.with_subject(subject))
}

/// One leading option word the call supplied, as the relation checker needs
/// it: the canonical name, the flag word's span, and the option's first value
/// word when that word is statically known.
#[derive(Debug, Clone)]
pub(in crate::analyser) struct SeenOption {
    pub(in crate::analyser) name: &'static str,
    pub(in crate::analyser) span: tcl_lexer::Span,
    pub(in crate::analyser) value: Option<String>,
}

/// One positional word after the leading option run: its literal text when
/// statically known, and its span.
#[derive(Debug, Clone)]
pub(in crate::analyser) struct SeenPositional {
    pub(in crate::analyser) value: Option<String>,
    pub(in crate::analyser) span: tcl_lexer::Span,
}

/// Selected source options and positionals retained together for relationship
/// verdicts, including the independent completeness obligation.
#[derive(Debug, Default)]
pub(in crate::analyser) struct ScannedInvocation {
    pub(in crate::analyser) options: Vec<SeenOption>,
    pub(in crate::analyser) positionals: Vec<SeenPositional>,
    /// Whether the call was read to its end with every word statically known.
    /// **Only** this licenses proving a relation term *absent*.
    pub(in crate::analyser) complete: bool,
}

/// The span a relation violation points at: the run from the first to the last
/// word the violation names, or `fallback` when it names none the call
/// actually supplied (a `requires-one-of` that found nothing).
fn relation_span(
    present: &[tcl_registry::OptionTerm],
    seen_options: &[SeenOption],
    positionals: &[SeenPositional],
    fallback: tcl_lexer::Span,
) -> tcl_lexer::Span {
    let span_of = |term: tcl_registry::OptionTerm| -> Option<tcl_lexer::Span> {
        match term {
            tcl_registry::OptionTerm::Option(name)
            | tcl_registry::OptionTerm::OptionValue(name, _) => seen_options
                .iter()
                .find(|option| option.name == name)
                .map(|option| option.span),
            tcl_registry::OptionTerm::Argument(index)
            | tcl_registry::OptionTerm::ArgumentValue(index, _) => {
                positionals.get(usize::from(index)).map(|word| word.span)
            }
        }
    };
    let spans: Vec<tcl_lexer::Span> = present.iter().copied().filter_map(span_of).collect();
    match (
        spans.iter().map(|span| span.start()).min(),
        spans.iter().map(|span| span.end()).max(),
    ) {
        (Some(start), Some(end)) => tcl_lexer::Span::new(start, end),
        _ => fallback,
    }
}

/// **The whole E-R14 consumer**, shared by the ordinary command path and the
/// object-instance dispatch path so one invocation is judged by one rule.
///
/// Every relation is evaluated natively; the `constraints` hook is reached
/// only when a spec declares one *and* nothing declarative had anything to
/// report — so a registry with no `constraints` hook (which is every shipped
/// command) never enters the VM at any call site.
///
/// Each entry pairs the relation's [`Lifecycle`] with its diagnostic, so the
/// caller can buffer a version-gated relation for the post-walk floor and push
/// an ungated one inline.
pub(in crate::analyser) fn option_relation_diagnostics(
    display_name: &str,
    relations: &[&'static tcl_registry::OptionRelation],
    constraints_hook: Option<tcl_registry::ConstraintsHook>,
    call: &ScannedInvocation,
    fallback_span: tcl_lexer::Span,
) -> Vec<(Lifecycle, crate::analyser::types::Diagnostic)> {
    let seen_options = &call.options;
    let positionals = &call.positionals;
    if relations.is_empty() && constraints_hook.is_none() {
        tcl_registry::spec::note_relation_check(!seen_options.is_empty(), false, 0, false);
        return Vec::new();
    }
    let options: Vec<(&'static str, Option<&str>)> = seen_options
        .iter()
        .map(|option| (option.name, option.value.as_deref()))
        .collect();
    let positional_values: Vec<Option<&str>> = positionals
        .iter()
        .map(|word| word.value.as_deref())
        .collect();
    let facts = tcl_registry::OptionFacts {
        options: &options,
        positionals: &positional_values,
        complete: call.complete,
    };

    let mut out = Vec::new();
    for relation in relations {
        let tcl_registry::RelationVerdict::Violated(violation) = relation.evaluate(&facts) else {
            continue;
        };
        let span = relation_span(&violation.present, seen_options, positionals, fallback_span);
        let code = if violation.kind.is_exclusion() {
            DiagCode::W147
        } else {
            DiagCode::W152
        };
        out.push((
            relation.lifecycle,
            crate::analyser::types::Diagnostic::new(
                code,
                span,
                violation.message_for(display_name),
                Severity::Warning,
            ),
        ));
    }

    // The escape hatch, and only here: a spec that declares no `constraints`
    // hook never reaches the hook seam at all, and one that does is asked only
    // when the declarative relations found nothing to report.
    if !out.is_empty() {
        tcl_registry::spec::note_relation_check(
            !seen_options.is_empty(),
            true,
            relations.len(),
            false,
        );
        return out;
    }
    let Some(hook) = constraints_hook else {
        tcl_registry::spec::note_relation_check(
            !seen_options.is_empty(),
            true,
            relations.len(),
            false,
        );
        return out;
    };
    tcl_registry::spec::note_relation_check(!seen_options.is_empty(), true, relations.len(), true);
    for report in hook(&facts) {
        let span = match report.slot {
            tcl_registry::ConstraintSlot::Option(name) => seen_options
                .iter()
                .find(|option| option.name == name)
                .map_or(fallback_span, |option| option.span),
            tcl_registry::ConstraintSlot::Argument(index) => positionals
                .get(usize::from(index))
                .map_or(fallback_span, |word| word.span),
            tcl_registry::ConstraintSlot::Command => fallback_span,
        };
        let code = if report.conflict {
            DiagCode::W147
        } else {
            DiagCode::W152
        };
        out.push((
            Lifecycle::UNSPECIFIED,
            crate::analyser::types::Diagnostic::new(code, span, report.message, Severity::Warning),
        ));
    }
    out
}

/// Whether `word` is a literal list value: braced, or free of outer Tcl
/// substitutions. A braced word is literal even when its eventual elements
/// contain `$` / `[`. A computed value can be valid or invalid at runtime, so
/// reporting it would be an unsound claim.
fn is_literal_list_word(word: &str, token: &tcl_lexer::Token) -> bool {
    token.kind == tcl_lexer::TokenType::Str
        || (token.kind != tcl_lexer::TokenType::Expand && !has_substitution(word, token))
}

/// Emit E006 for every registry-identified, statically-known formal-parameter
/// list that Tcl would reject while creating its callable.  Callers supply the
/// role-derived indices so this routine deliberately has no knowledge of the
/// command or definition-body member that owns the list.
pub(in crate::analyser) fn emit_invalid_formal_parameter_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    parameter_indices: &[usize],
) {
    let Some(grammar) = tcl_registry::InvocationDialect::of_profile(
        analyser.resolved_analysis_input().unit_profile(),
    )
    .parameter_grammar() else {
        return;
    };
    for &index in parameter_indices {
        let (Some(params), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if !is_literal_list_word(params, token) {
            continue;
        }
        let Err(error) = crate::signature_scan::params::parse_param_list_strict_in(
            params,
            analyser.word_rules(),
            grammar,
        ) else {
            continue;
        };
        let span = super::super::utils::full_word_span(*token, &analyser.source);
        let fixes = tcl_syntax::formal_params::split_overlong_parameter_specifier(params, &error)
            .and_then(|repaired| {
                let original = analyser.retained_value_word(*token)?;
                (original.span() == span
                    && tcl_syntax::word_rules::original_static_word_unicode_value(&original)?
                        .as_slice()
                        == params.as_bytes())
                .then_some(())?;
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &repaired,
                    original.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    "Split the grouped fields into separate parameters",
                )])
            })
            .unwrap_or_default();
        analyser.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::E006,
                span,
                format!("Invalid formal parameter list: {error}"),
                Severity::Error,
            )
            .with_fixes(fixes),
        );
    }
}

/// E006 for a registry-identified ([`tcl_registry::ArgRole::StaticVarList`]),
/// statically-known static-variable list with an element of more than two
/// fields: a static is a name, a `{name value}` pair or `&name`, and `jimsh`
/// rejects any other shape while creating the procedure.
pub(in crate::analyser) fn emit_invalid_static_variable_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    static_indices: &[usize],
) {
    for &index in static_indices {
        let (Some(statics), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if !is_literal_list_word(statics, token) {
            continue;
        }
        let word_rules = analyser.word_rules();
        let Ok(elements) = word_rules.split_list(statics) else {
            continue;
        };
        let too_many = elements.iter().find(|element| {
            word_rules
                .split_list(element)
                .is_ok_and(|fields| fields.len() > 2)
        });
        let Some(element) = too_many else {
            continue;
        };
        let span = super::super::utils::full_word_span(*token, &analyser.source);
        analyser.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
            DiagCode::E006,
            span,
            format!("Invalid static variable list: too many fields in static specifier \"{element}\""),
            Severity::Error,
        ));
    }
}

/// E006's lambda-literal companion.  `ArgRole::LambdaLiteral` declares the
/// standard `{parameters body ?namespace?}` value shape, so a consumer can
/// discover the nested parameter list without knowing the command that owns
/// the literal.
pub(in crate::analyser) fn emit_invalid_lambda_parameter_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    lambda_indices: &[usize],
) {
    let Some(grammar) = tcl_registry::InvocationDialect::of_profile(
        analyser.resolved_analysis_input().unit_profile(),
    )
    .parameter_grammar() else {
        return;
    };
    for &index in lambda_indices {
        let (Some(lambda), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if token.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let word_rules = analyser.word_rules();
        let Ok(fields) = word_rules.split_list(lambda) else {
            continue;
        };
        let Some(params) = fields.first() else {
            continue;
        };
        let Err(error) =
            crate::signature_scan::params::parse_param_list_strict_in(params, word_rules, grammar)
        else {
            continue;
        };
        let span = super::super::utils::full_word_span(*token, &analyser.source);
        let fixes = tcl_syntax::formal_params::split_overlong_parameter_specifier(params, &error)
            .and_then(|repaired| {
                let original = analyser.retained_value_word(*token)?;
                (original.span() == span
                    && tcl_syntax::word_rules::original_static_word_unicode_value(&original)?
                        .as_slice()
                        == lambda.as_bytes())
                .then_some(())?;
                let mut repaired_fields = fields.clone();
                repaired_fields[0] = repaired.into();
                let repaired_lambda = tcl_syntax::list::join_list(repaired_fields);
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &repaired_lambda,
                    original.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    "Split the grouped fields into separate parameters",
                )])
            })
            .unwrap_or_default();
        analyser.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::E006,
                span,
                format!("Invalid formal parameter list: {error}"),
                Severity::Error,
            )
            .with_fixes(fixes),
        );
    }
}

/// Describe [`Arity`]'s [`Arity::step`] / [`Arity::also_exact`] shape in
/// prose for the E005 message — `"0, 2, 4, …"` for a plain progression,
/// `"2, or 3, 5, 7, …"` when `also_exact` adds an exception (`switch`'s
/// shorthand-or-pairs union). Only called when `step != 0` (a caller with
/// no parity constraint never reaches the E005 branch).
fn describe_step_shape(arity: Arity) -> String {
    let min = u32::from(arity.min);
    let step = u32::from(arity.step);
    let progression = format!("{min}, {}, {}, …", min + step, min + step * 2);
    match arity.also_exact {
        Some(exact) => format!("{exact}, or {progression}"),
        None => progression,
    }
}

/// Compare a resolved [`Arity`] against an observed positional-argument
/// count and build the E002 / E003 / E005 diagnostic, or `None` when the
/// count fits. Shared by the registry-command arity path
/// ([`Analyser::emit_source_signature_arity`]), the same-file proc / `TclOO`
/// method / `interp alias` / `rename` arity path
/// ([`Analyser::flush_arity_diagnostics`]), and the `TclOO` method-call
/// arity check ([`super::var_command`]), so all three diagnostics carry
/// identical wording.
///
/// **E005** fires when `nargs_min` is within `[min, max]` (so neither
/// E002 nor E003 applies) but doesn't land on [`Arity::step`]'s
/// progression or match [`Arity::also_exact`] — a count that's
/// individually "enough" but doesn't fit the command's key/value-pair /
/// paired-argument shape (`dict create a` — one word, not a key/value
/// pair; `foreach a b c d` — an unpaired trailing var-list). Same
/// `{*}`-expansion abstention as E002: an expanded tail's true final
/// count is unknowable, so a merely-in-range lower bound proves nothing
/// about its parity either.
///
/// `usage` is the resolved spec's primary synopsis; when present it is
/// appended to every verdict as a " — usage: …" suffix so the message
/// shows the expected call shape, not just the counts. Callers with no
/// registry spec (same-file procs, `TclOO` constructors/methods, `apply`
/// lambdas) pass `None` and keep the count-only message.
pub(super) fn arity_verdict(
    display_name: &str,
    arity: Arity,
    nargs_min: usize,
    positional_any_expand: bool,
    span: tcl_lexer::Span,
    excess: Option<ExcessArgs>,
    usage: Option<&str>,
) -> Option<crate::analyser::types::Diagnostic> {
    let usage_suffix = usage.map(|u| format!(" — usage: {u}")).unwrap_or_default();
    let min = usize::from(arity.min);
    let max = usize::from(arity.max);
    // `also_exact` is valid regardless of `min`/`max`/`step` — checked
    // first so it exempts the shorthand count from every branch below,
    // not just E005 (`switch $s {p1 b1 p2 b2}`'s 2-arg form must not
    // trip E002 just because `switch`'s own `min` is 3).
    if arity
        .also_exact
        .is_some_and(|exact| usize::from(exact) == nargs_min)
    {
        return None;
    }
    if !positional_any_expand && nargs_min < min {
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E002,
            span,
            format!(
                "Too few arguments for '{display_name}': expected at least {min}, \
got {nargs_min}{usage_suffix}"
            ),
            Severity::Error,
        ))
    } else if !arity.is_unlimited() && nargs_min > max {
        // Highlight only the surplus arguments when the caller could
        // isolate them (the whole command otherwise), and offer to delete
        // exactly that run (plus the separating whitespace). E002/E005 keep
        // the whole-command anchor: a too-few or wrong-shape count has no
        // specific surplus word to point at.
        let (e003_span, fixes) = match &excess {
            Some(ex) => (
                ex.span,
                vec![crate::analyser::types::CodeFix {
                    span: tcl_lexer::Span::new(ex.delete_from, ex.span.end()),
                    new_text: String::new(),
                    description: "Remove surplus argument(s)".to_string(),
                    // E003: deleting the surplus words assumes they were the mistake
                    // rather than a missing option that would have consumed them.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }],
            ),
            None => (span, Vec::new()),
        };
        Some(
            crate::analyser::types::Diagnostic::new(
                DiagCode::E003,
                e003_span,
                format!(
                    "Too many arguments for '{display_name}': expected at most {max}, \
got {nargs_min}{usage_suffix}"
                ),
                Severity::Error,
            )
            .with_fixes(fixes),
        )
    } else if !positional_any_expand
        && arity.step != 0
        && !(nargs_min - min).is_multiple_of(usize::from(arity.step))
    {
        Some(crate::analyser::types::Diagnostic::new(
            DiagCode::E005,
            span,
            format!(
                "Wrong argument-count shape for '{display_name}': expected {}, \
got {nargs_min}{usage_suffix}",
                describe_step_shape(arity)
            ),
            Severity::Error,
        ))
    } else {
        None
    }
}

/// Format the "(available in: …)" suffix for a W002 message from a
/// command/subcommand's registry-declared dialect restriction — entirely
/// data-driven from [`tcl_registry::CommandSpec::surface`] /
/// [`tcl_registry::SubCommand::surface`], never a per-command name check.
/// Empty when `surface` is `None` (unrestricted) or has no primitive
/// member (defensive; a restricted spec always has at least one).
///
/// Members print as the profile catalogue's `short_name` ("Tcl 8.5",
/// "iRules") — this suffix is the highest-traffic dialect naming in
/// editor-visible prose, so it uses the human spelling; a member with no
/// catalogue profile keeps its canonical name.
fn dialect_availability_suffix(surface: Option<&'static [SpecSurface]>) -> String {
    surface.map_or_else(String::new, dialect_availability_suffix_for_rows)
}

/// [`dialect_availability_suffix`] over surface rows the caller already holds.
fn dialect_availability_suffix_for_rows(rows: &[SpecSurface]) -> String {
    // The dialect ids the rows name, in the registry's own projection —
    // the reader wants the dialects they can select, not the row spelling.
    let labels: Vec<String> = tcl_registry::model::surface::dialect_names_for_rows(rows)
        .into_iter()
        .map(|name| {
            tcl_dialect::DialectProfile::find(&name)
                .map_or(name.clone(), |profile| profile.short_name.to_owned())
        })
        .collect();
    if labels.is_empty() {
        return String::new();
    }
    format!(" (available in: {})", labels.join(", "))
}

/// Reporting candidates plus the **implicit `namespace path`** in force at
/// `call_off` — today only a `TclOO` member body's `::oo::Helpers`, and that
/// name comes from the registry
/// ([`tcl_registry::definer::TCLOO_MEMBER_BODY_NAMESPACE_PATH`], read through
/// [`crate::analyser::scope::implicit_command_namespace_path_at`]), never from
/// a literal here.
///
/// Without the path a bare word written in a method body only ever qualifies
/// to `::WORD`, so a document that installs its own
/// `proc ::oo::Helpers::callback` — the documented "`TclOO` Tricks" wiki
/// helper `ticklecharts` uses under 8.6 — would not count as shadowing the
/// like-named 9.0 builtin, and the call would draw a W002 "disabled in the
/// active dialect profile" the program provably never sees. tclsh 8.6.14 runs
/// that file: the helper *is* the command the method body reaches.
fn qualify_candidates_with_path(ns: &str, cmd_name: &str, path: &[&str]) -> Vec<String> {
    crate::naming::command_resolution_candidates(ns, path, cmd_name)
}

/// Deferred builtin/private-advice suppression over the merged file.
/// Actual user-call signature selection uses its retained original call owner.
/// Whole-command exclusions use their own typed original publication geometry;
/// this compatibility projection supplies no Native naming or handler proof.
///
/// Owns its data (rather than borrowing `Analyser`) so a flush loop can
/// build it once up front and then freely take/mutate other `Analyser`
/// fields (`pending_arity`, `result.diagnostics`, …) while iterating —
/// borrowing through a whole-`&Analyser` build call would otherwise pin the
/// borrow checker's view to "all of `self`" for the facts' lifetime.
pub(super) struct UserResolutionFacts {
    /// Fully-qualified names that unconditionally denote a user command once
    /// declared anywhere in the file: `TclOO`/snit/itcl classes, `interp
    /// alias` names, and `namespace ensemble create` namespaces. Not
    /// order-gated — a deliberately permissive, false-negative-avoiding
    /// convention (a genuinely order-violating call is rare enough that
    /// abstaining from a real builtin-mismatch report is the safer trade).
    non_proc_qnames: FxHashSet<String>,
    /// Qualified proc name → the offset of its *earliest* declaration —
    /// the point from which the name denotes a user command at all. A name
    /// declared twice keeps the first offset here (`all_procs` alone only
    /// remembers the second), so a call between the two still suppresses the
    /// builtin-mismatch diagnostic it shadows.
    proc_offsets: FxHashMap<String, u32>,
    /// Qualified *new* name (a static `rename OLD NEW`) → the `rename`
    /// statement's token offset. `rename` moves an existing command's
    /// identity onto `NEW`, so once in effect `NEW` is a real command
    /// exactly like a proc definition — order-gated the same way.
    rename_offsets: FxHashMap<String, u32>,
    /// Document-global `# tcl-lsp: stub` command names (unqualified).
    stub_names: std::collections::HashSet<String>,
}

impl UserResolutionFacts {
    pub(super) fn build(a: &Analyser) -> Self {
        let mut non_proc_qnames: FxHashSet<String> = FxHashSet::default();
        non_proc_qnames.extend(a.result.all_classes.keys().cloned());
        non_proc_qnames.extend(a.result.command_aliases.keys().cloned());
        non_proc_qnames.extend(a.ensemble_namespaces.iter().cloned());
        let proc_offsets: FxHashMap<String, u32> = a
            .result
            .all_procs
            .keys()
            .map(|qname| {
                let earliest = a
                    .result
                    .proc_declarations(qname)
                    .map(|def| def.name_span.start())
                    .min()
                    .unwrap_or(u32::MAX);
                (qname.clone(), earliest)
            })
            .collect();
        let rename_offsets: FxHashMap<String, u32> = a
            .result
            .renamed_commands
            .keys()
            .filter_map(|new_qname| {
                a.rename_offsets
                    .get(new_qname)
                    .map(|&off| (new_qname.clone(), off))
            })
            .collect();
        let stub_names = super::utils::scan_stub_command_names(&a.source);
        Self {
            non_proc_qnames,
            proc_offsets,
            rename_offsets,
            stub_names,
        }
    }

    /// Whether a call to `cmd_name` (resolved at `ns`, through the implicit
    /// `namespace path` `path`, at byte offset `call_off`) resolves to a
    /// user-declared proc, class, alias, rename target, ensemble, or stub,
    /// rather than falling through to the registry builtin that produced the
    /// candidate diagnostic.
    ///
    /// `enforce_order` gates **proc** and **rename** shadowing to a
    /// definition that lexically precedes the call: true for a top-level
    /// call site (the module body, a `namespace eval` body, or a
    /// conditional), which executes in source order during load, so a
    /// same-named definition appearing later in the file has not run yet. A
    /// proc-body call (`enforce_order == false`) runs only after the whole
    /// file has loaded, so any same-named definition anywhere in the file is
    /// already in effect. Classes / aliases / ensembles / stubs always exist
    /// by run time and are never order-gated.
    fn resolves_to_user(
        &self,
        cmd_name: &str,
        ns: &str,
        path: &[&str],
        enforce_order: bool,
        call_off: u32,
    ) -> bool {
        let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
            cmd_name.as_bytes(),
        ))
        .expect("tail of a Unicode command");
        let candidates = qualify_candidates_with_path(ns, cmd_name, path);
        candidates.iter().any(|c| {
            self.non_proc_qnames.contains(c.as_str())
                || self
                    .proc_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
                || self
                    .rename_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
        }) || self.stub_names.contains(bare)
    }
}

/// Shift a resolved [`Arity`] down by an `interp alias` / `TclOO`
/// `forward`'s prepended-argument count — real Tcl partial application
/// (confirmed against tclsh 9.0.4: `interp alias {} short {} target
/// extra` requires exactly `target`'s arity minus one fewer argument at
/// the `short` call site).
///
/// When the prepended count already exceeds a *bounded* target's own
/// max, the alias/forward is unconditionally broken — every call fails
/// at run time regardless of how many further arguments are supplied
/// (confirmed against tclsh 9.0.4: `proc target {a} {}; interp alias {}
/// bad {} target fixed extra; bad` fails "wrong # args" for zero, one,
/// or two further arguments alike). Saturating both bounds to zero would
/// misrepresent that as "callable with exactly zero arguments" — a
/// silent false negative. Returning an unsatisfiable range (`min > max`)
/// instead guarantees `arity_verdict` flags every call, whatever count
/// it's called with.
pub(super) fn shift_arity(arity: Arity, prepended: u16) -> Arity {
    if !arity.is_unlimited() && prepended > arity.max {
        return Arity::new(1, 0);
    }
    let min = arity.min.saturating_sub(prepended);
    let max = if arity.is_unlimited() {
        Arity::UNLIMITED
    } else {
        arity.max.saturating_sub(prepended)
    };
    Arity::new(min, max)
}

/// Family advice retains the Native canonical definer and explicitly Logical
/// source model as separate admission paths. A report-map default alone grants
/// neither family classification nor current object dispatch.
pub(super) fn is_tcloo_source_class(analyser: &Analyser, class: &super::types::ClassDef) -> bool {
    if class.source_name_ambiguous.is_observed() {
        return false;
    }
    let analysis = &analyser.result;
    if analysis.allows_retained_logical_declaration_advice() {
        return analyser
            .retained_logical_class_definer_grammar(&class.qualified_name)
            .is_some_and(|grammar| grammar.family == tcl_registry::definer::DefinerFamily::TclOo);
    }
    let source = &analyser.source;
    let Some(publication) = class.source_name.as_ref() else {
        return false;
    };
    let mut records = analysis.original_class_declarations().filter(|record| {
        record.name() == publication && record.name_input().span() == class.name_span
    });
    let Some(record) = records.next() else {
        return false;
    };
    if records.next().is_some() {
        return false;
    }
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    crate::command_binding::OriginalSourceClassDeclaration::from_class(source, analysis, record)
        .and_then(|declaration| declaration.grammar(&input.context_registry()))
        .is_some_and(|grammar| grammar.family == tcl_registry::definer::DefinerFamily::TclOo)
}

/// Deferred original literal-head source policy. It retains the real word and
/// complete input; it never asserts private implementation execution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::analyser) struct PendingPrivateTclSourceAdvice {
    subject: std::sync::Arc<crate::analyser::SourcePrivateTclNamespaceSubject>,
    diagnostic: super::types::Diagnostic,
}

impl Analyser {
    /// **W146.** Run the resolved invocation's registry-owned relationship or
    /// literal-content validator. The analyser knows only how to project
    /// source words, anchor the returned argument index, and render the typed
    /// issue; every legal set and cross-argument relationship stays in the
    /// command registry.
    pub(in crate::analyser) fn emit_w146_literal_argument_validation(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(tcl_registry::LiteralArgumentValidation::Invalid(issue)) = original
            .with_schema(crate::analyser::diagnostic_registry::source_literal_validation)
            .flatten()
        else {
            return;
        };
        let Some(operand) = original.word(issue.argument_index) else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::RegistrySourceDiagnosticKind::LiteralArgument,
            Some(issue.argument_index),
        ) else {
            return;
        };
        let span = operand.span();
        let allowed = issue.allowed_values.join(", ");
        let (message, invalid_description) = match &issue.reason {
            tcl_registry::LiteralArgumentIssueReason::Empty => (
                format!(
                    "Empty {}; expected one or more of: {allowed}",
                    issue.subject
                ),
                String::new(),
            ),
            tcl_registry::LiteralArgumentIssueReason::InvalidMembers(invalid) => {
                let quoted = invalid
                    .iter()
                    .map(|member| format!("'{member}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                (
                    format!(
                        "Invalid member(s) {quoted} in {}; expected one or more of: {allowed}",
                        issue.subject
                    ),
                    quoted,
                )
            }
        };
        let fixes = issue
            .replacement_value
            .and_then(|replacement| {
                original.written_index(issue.argument_index)?;
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &replacement,
                    operand.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    format!("Remove invalid member(s) {invalid_description}"),
                )])
            })
            .unwrap_or_default();
        let diagnostic = crate::analyser::types::Diagnostic::new(
            DiagCode::W146,
            span,
            message,
            Severity::Warning,
        )
        .with_fixes(fixes);
        self.result
            .diagnostics
            .push(diagnostic.with_subject(subject));
    }

    /// Whole-command availability advice from retained original source.
    /// Later original declarations can withdraw nominal Registry metadata.
    pub(in crate::analyser) fn emit_w002_disabled_command(
        &mut self,
        original: Option<crate::registry_invocation::OriginalSourceCommandAvailability>,
        scope_path: &[usize],
    ) {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        use tcl_registry::model::CommandSourceUnavailabilityKind;
        let Some(original) = original else {
            return;
        };
        let descriptor = original.descriptor();
        let reporting_name = original
            .original_head()
            .try_text()
            .unwrap_or_default()
            .to_owned();
        let (code, message) = match descriptor.kind() {
            CommandSourceUnavailabilityKind::RuleLoaderRefused => (
                DiagCode::Irule2004,
                format!(
                    "'{reporting_name}' is refused by the authored iRules rule-loader surface when written literally in rule source."
                ),
            ),
            CommandSourceUnavailabilityKind::Unavailable => {
                let suffix = dialect_availability_suffix_for_rows(&descriptor.providers().rows);
                (
                    DiagCode::W002,
                    format!(
                        "Registry command '{reporting_name}' is disabled in the active dialect profile{suffix}"
                    ),
                )
            }
        };
        let diag = crate::analyser::types::Diagnostic::new(
            code,
            original.original_head().span(),
            message,
            Severity::Warning,
        )
        .with_subject(crate::analyser::DiagnosticSubject::CommandAvailability(
            std::sync::Arc::new(original),
        ));
        let ns = self.command_resolution_namespace(scope_path);
        let enforce_order = !self.scope_path_in_proc_body(scope_path);
        self.pending_disabled_commands
            .push((reporting_name, ns, enforce_order, diag));
    }

    /// W001, W002 and W145 selector diagnostics from original source schema.
    /// The retained context owns identity, availability and prefix resolution;
    /// a captured, dynamic or expanded selector has no written subject here.
    /// Suggestions replace the complete original selector word and need review.
    pub(in crate::analyser) fn emit_w001_unknown_subcommand(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_subcommand_diagnostic,
        };
        use tcl_registry::AuthoredSourceSubcommandDiagnostic;

        let Some(original) = original else {
            return;
        };
        let Some(word) = original.literal(0) else {
            return;
        };
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::Subcommand, Some(0))
        else {
            return;
        };
        let Some(outcome) = original.with_schema(source_subcommand_diagnostic).flatten() else {
            return;
        };
        let command = original.command();
        let span = original
            .word(0)
            .expect("subject retains original word")
            .span();
        let (code, message, fixes) = match outcome {
            AuthoredSourceSubcommandDiagnostic::Disabled { canonical, surface } => (
                DiagCode::W002,
                format!(
                    "'{command} {canonical}' is disabled in the active dialect profile{}",
                    dialect_availability_suffix(surface)
                ),
                Vec::new(),
            ),
            AuthoredSourceSubcommandDiagnostic::Unknown { candidates } => {
                let mut message = format!("Unknown subcommand '{word}' for '{command}'");
                let suggestions = crate::text::suggest_similar(
                    word,
                    candidates.iter().copied(),
                    1,
                    crate::text::scaled_max_distance(word),
                );
                let fixes = suggestions
                    .first()
                    .and_then(|best| {
                        use std::fmt::Write as _;
                        let _ = write!(message, "; did you mean '{best}'?");
                        original.written_index(0)?;
                        let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                            best,
                            original.word(0)?.config(),
                        )?;
                        Some(vec![super::types::CodeFix {
                            span,
                            new_text,
                            description: format!("Replace with '{best}'"),
                            safety: crate::irules_checks::FixSafety::RequiresReview,
                        }])
                    })
                    .unwrap_or_default();
                (DiagCode::W001, message, fixes)
            }
            AuthoredSourceSubcommandDiagnostic::Ambiguous { candidates } => {
                let listed = candidates
                    .iter()
                    .map(|candidate| format!("'{candidate}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let fixes = candidates
                    .into_iter()
                    .filter_map(|candidate| {
                        original.written_index(0)?;
                        let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                            candidate,
                            original.word(0)?.config(),
                        )?;
                        Some(super::types::CodeFix {
                            span,
                            new_text,
                            description: format!("Expand to '{command} {candidate}'"),
                            safety: crate::irules_checks::FixSafety::RequiresReview,
                        })
                    })
                    .collect();
                (
                    DiagCode::W145,
                    format!("Ambiguous abbreviation '{word}' for '{command}': matches {listed}."),
                    fixes,
                )
            }
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(code, span, message, Severity::Warning)
                .with_fixes(fixes)
                .with_subject(subject),
        );
    }

    pub(in crate::analyser) fn emit_proven_source_option_relationships(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        self.emit_source_option_relationships(original, display_name);
    }

    /// Original Registry signatures use the retained source selection and
    /// shared count axes. User calls, object calls and lambda parameter lists
    /// retain their own separate declaration owners.
    pub(in crate::analyser) fn emit_arity_diagnostics(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        self.queue_user_call_arity_candidate(cmd_name, cmd_tok);
        self.queue_tcloo_arity_candidates(cmd_name, cmd_tok, scope_path);
        let Some(original) = original else {
            return;
        };
        self.emit_source_lambda_arity(original, cmd_name);
        self.emit_source_option_relationships(original, cmd_name);
        self.emit_source_signature_arity(original, cmd_name);
    }

    /// Descriptive declaration arity remains distinct from builtin signatures.
    /// The sealed issuer validates direct-written geometry and all lookup
    /// barriers; no Registry option, concatenation or dispatch fact is donated.
    pub(in crate::analyser) fn emit_declared_source_arity(
        &mut self,
        original: crate::command_binding::OriginalDeclaredCommandWords,
        display_name: &str,
    ) {
        use crate::analyser::diagnostic_registry::{
            DeclaredSourceDiagnosticKind, DeclaredSourceDiagnosticSubject,
        };
        let Some(arity) = original.source_arity() else {
            return;
        };
        let words = original.original_words();
        let (Some(first), Some(last)) = (words.first(), words.last()) else {
            return;
        };
        let span = tcl_lexer::Span::new(first.span().start(), last.span().end());
        let max = usize::from(arity.max);
        let excess = words
            .get(max.saturating_add(1))
            .zip(words.get(max))
            .map(|(surplus, kept)| ExcessArgs {
                span: tcl_lexer::Span::new(surplus.span().start(), last.span().end()),
                delete_from: kept.span().end(),
            });
        let Some(diagnostic) = arity_verdict(
            display_name,
            arity,
            original.arguments().len(),
            false,
            span,
            excess,
            None,
        ) else {
            return;
        };
        let Some(subject) = DeclaredSourceDiagnosticSubject::at_extent(
            std::sync::Arc::new(original),
            DeclaredSourceDiagnosticKind::Arity,
            diagnostic.span,
        ) else {
            return;
        };
        self.result
            .diagnostics
            .push(diagnostic.with_subject(subject));
    }

    fn emit_source_signature_arity(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        use crate::analyser::diagnostic_registry::source_arity;
        let Some(shape) = original.with_schema(source_arity).flatten() else {
            return;
        };
        let axis = self.lifecycle_axis(shape.command);
        if shape.missing_subcommand {
            if !shape.windows.is_empty() {
                self.pending_gated_bare_ensemble
                    .push(super::version_gate::GatedBareEnsemble {
                        axis,
                        windows: shape.windows,
                        fallback: shape.arity,
                        display_name: display_name.to_owned(),
                        original: original.clone(),
                    });
            } else if shape.arity.min > 0 {
                self.queue_source_arity_diagnostic(
                    original,
                    crate::analyser::types::Diagnostic::new(
                        DiagCode::E001,
                        original.head().span(),
                        format!("'{display_name}' requires a subcommand"),
                        Severity::Error,
                    ),
                );
            }
            return;
        }
        let display_name = shape.subcommand.map_or_else(
            || display_name.to_owned(),
            |sub| format!("{display_name} {}", sub.name),
        );
        if !shape.windows.is_empty() {
            self.pending_gated_arity
                .push(super::version_gate::GatedArityCall {
                    axis,
                    windows: shape.windows,
                    fallback: shape.arity,
                    display_name,
                    original: original.clone(),
                    synopsis: shape.synopsis,
                });
        } else if let Some(diagnostic) = source_arity_verdict(
            original,
            &display_name,
            shape.arity,
            &shape.count,
            shape.synopsis,
        ) {
            self.queue_source_arity_diagnostic(original, diagnostic);
        }
    }

    pub(super) fn queue_source_arity_diagnostic(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        diagnostic: crate::analyser::types::Diagnostic,
    ) {
        let Some(subject) = original.subject_extent(
            crate::analyser::RegistrySourceDiagnosticKind::Arity,
            diagnostic.span,
        ) else {
            return;
        };
        self.pending_arity.push((
            original.command().to_owned(),
            String::new(),
            false,
            diagnostic.with_subject(subject),
        ));
    }

    fn emit_source_option_relationships(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        use crate::analyser::diagnostic_registry::source_option_relationships;
        let Some(facts) = original.with_schema(source_option_relationships).flatten() else {
            return;
        };
        if facts.relations.is_empty() && facts.constraints.is_none() {
            return;
        }
        let fallback = original.head().span();
        let value = |argument: usize| {
            original
                .words()
                .arguments()
                .get(argument)
                .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
        };
        let call = ScannedInvocation {
            options: facts
                .scan
                .options
                .iter()
                .filter(|option| option.available)
                .map(|option| SeenOption {
                    name: option.option.name,
                    span: original
                        .word(option.argument)
                        .map_or(fallback, tcl_lexer::NativeWord::span),
                    value: option
                        .values
                        .as_ref()
                        .and_then(|values| value(values.start)),
                })
                .collect(),
            positionals: facts
                .positionals
                .iter()
                .map(|&argument| SeenPositional {
                    value: value(argument),
                    span: original
                        .word(argument)
                        .map_or(fallback, tcl_lexer::NativeWord::span),
                })
                .collect(),
            complete: facts.complete,
        };
        let axis = original
            .with_schema(crate::analyser::diagnostic_registry::source_descriptors)
            .and_then(|selected| self.lifecycle_axis(selected.command));
        for (lifecycle, diagnostic) in option_relation_diagnostics(
            display_name,
            &facts.relations,
            facts.constraints,
            &call,
            fallback,
        ) {
            let Some(subject) = original.subject_extent(
                crate::analyser::RegistrySourceDiagnosticKind::ArgumentRelationship,
                diagnostic.span,
            ) else {
                continue;
            };
            let diagnostic = diagnostic.with_subject(subject);
            if lifecycle.is_unspecified() {
                self.pending_arity.push((
                    original.command().to_owned(),
                    String::new(),
                    false,
                    diagnostic,
                ));
            } else {
                self.pending_option_conflicts
                    .push(super::version_gate::GatedOptionConflict {
                        axis,
                        lifecycle,
                        resolution_name: original.command().to_owned(),
                        namespace: String::new(),
                        enforce_order: false,
                        diagnostic,
                    });
            }
        }
    }

    /// A lambda call's count belongs to its original selected Apply grammar.
    /// The lambda value is parsed as a list and strict parameter list under
    /// the retained dialect; lambda data is never segmented as a script.
    fn emit_source_lambda_arity(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        display_name: &str,
    ) {
        use crate::analyser::diagnostic_registry::source_lambda_call;
        let Some(layout) = original.with_schema(source_lambda_call).flatten() else {
            return;
        };
        let Some(input) = original
            .words()
            .original_argument_value_input(layout.lambda_argument)
        else {
            return;
        };
        let Some(fields) = input.original_list_elements() else {
            return;
        };
        if !(2..=3).contains(&fields.len()) {
            return;
        }
        let Some(dialect) = original.words().dialect() else {
            return;
        };
        let Some(parameters) = crate::signature_scan::formal_parameters::SignatureSourceFormalParameterValue::from_original_input(&fields[0], dialect) else { return; };
        let Some(arity) =
            crate::signature_scan::arity::arity_from_count_shape(parameters.argument_count_shape())
        else {
            return;
        };
        if let Some(diagnostic) =
            source_arity_verdict(original, display_name, arity, &layout.count, None)
        {
            self.queue_source_arity_diagnostic(original, diagnostic);
        }
    }

    /// Publish current whole-command advice and legacy subcommand reports.
    /// Typed original publication geometry owns whole-command shadowing.
    pub(in crate::analyser) fn flush_disabled_command_diagnostics(&mut self) {
        if self.pending_disabled_commands.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);
        let pending = std::mem::take(&mut self.pending_disabled_commands);
        for (cmd_name, ns, enforce_order, diag) in pending {
            if let Some(original) = diag.command_availability() {
                if original.matches_analysis(&self.result)
                    && !original.has_source_declaration_shadow(&self.result, enforce_order)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    fn selected_source_package_is_unmentioned(
        &self,
        subject: &crate::analyser::RegistrySourceDiagnosticSubject,
    ) -> bool {
        let Some(package) = subject.owning_package() else {
            return false;
        };
        let Some(context) = subject.words().context() else {
            return true;
        };
        if context.ambient_package(package) || context.is_required(package) {
            return false;
        }
        let Some(head) = subject
            .words()
            .head_source()
            .and_then(|source| source.word())
        else {
            return true;
        };
        let policy = subject
            .words()
            .head_source()
            .and_then(|source| source.input())
            .map(crate::signature_scan::scope::SignatureSourceNameInput::policy);
        let matches = |name: &crate::signature_scan::original_name::SourcePackageName| {
            let Some(key) = name.input().original_word_key() else {
                return false;
            };
            let word = key.original_word();
            word.image() == head.image()
                && word.config() == head.config()
                && policy.is_some_and(|policy| policy == name.key().policy())
                && name.key().bytes() == package.as_bytes()
        };
        // These are authentic source applicability records, not package-table
        // installation or successful package loading observations.
        !self
            .result
            .package_requires
            .iter()
            .filter_map(|record| record.original_name.as_ref())
            .chain(
                self.result
                    .package_provides
                    .iter()
                    .filter_map(|record| record.original_name.as_ref()),
            )
            .any(matches)
    }

    /// Push each queued verdict about a builtin call whose call does not
    /// resolve to a user definition — the drain
    /// [`Self::flush_arity_diagnostics`] runs over `pending_arity`, shared
    /// with the proven-word pass, which settles its own verdicts after the
    /// flush.
    pub(super) fn settle_builtin_verdicts(
        &mut self,
        facts: &UserResolutionFacts,
        pending: Vec<(String, String, bool, crate::analyser::types::Diagnostic)>,
    ) {
        for (cmd_name, ns, enforce_order, diag) in pending {
            if let Some(subject) = diag.registry_source()
                && matches!(
                    subject.kind(),
                    crate::analyser::RegistrySourceDiagnosticKind::Arity
                        | crate::analyser::RegistrySourceDiagnosticKind::ArgumentRelationship
                )
            {
                // The sealed source selection owns naming/shadow applicability;
                // a reporting-name lookup cannot change that semantic purpose.
                if subject.kind() != crate::analyser::RegistrySourceDiagnosticKind::Arity
                    || !self.selected_source_package_is_unmentioned(subject)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            self.result.diagnostics.push(diag);
        }
    }

    /// Post-walk flush of the [`Self::pending_arity`] / [`Self::pending_user_call_arity`]
    /// candidates collected by [`Self::emit_arity_diagnostics`] and
    /// [`Self::queue_user_call_arity_candidate`].
    ///
    /// The [`Self::pending_arity`] half drops a candidate exactly when
    /// [`UserResolutionFacts::resolves_to_user`] says the call **resolves
    /// to** a user definition rather than the builtin whose registry arity
    /// produced it — resolution follows Tcl's rule for unqualified commands
    /// (the call-site namespace, then global `::`), using the namespace
    /// captured at emit time.  So `proc ::ns::close {...}` suppresses a
    /// `close` call inside `::ns` (and a qualified `::ns::close ...`), but a
    /// `close` call in another namespace still resolves to the builtin and
    /// is checked.  Document-global declarations — inline `# tcl-lsp:
    /// stub`s — suppress by bare name regardless of namespace.
    ///
    /// Suppression by a shadowing **proc** or **rename target** also honours
    /// definition reachability: a top-level call (one whose `enforce_order`
    /// flag is set — module body, `namespace eval` body, or a conditional)
    /// is silenced only when the definition lexically precedes it, since
    /// top-level commands run in source order during load (so a `close x y
    /// z` *before* a later `proc close` still reaches the builtin).
    /// Proc-body calls run after load and are not order-gated.  Classes /
    /// aliases / ensembles / stubs always exist at run time and are never
    /// order-gated.  (Excluding *conditionally* defined procs would need the
    /// CFG dominator model, which is not modelled here.)
    ///
    /// Idempotent: drains `pending_arity` and `pending_user_call_arity`,
    /// so a second call is a no-op.
    ///
    /// `pending_arity` carries E002/E003 arity candidates, W004
    /// (dialect-invalid-option) candidates, and W001 (unknown-subcommand)
    /// candidates (see its field doc); the *shadowing* suppression never
    /// inspects `diag.code`, so all three share it with no special-casing.
    pub fn flush_arity_diagnostics(&mut self) {
        if self.pending_arity.is_empty() && self.pending_user_call_arity.is_empty() {
            return;
        }
        let facts = UserResolutionFacts::build(self);

        let pending = std::mem::take(&mut self.pending_arity);
        for (cmd_name, ns, enforce_order, diag) in pending {
            if let Some(subject) = diag.registry_source()
                && matches!(
                    subject.kind(),
                    crate::analyser::RegistrySourceDiagnosticKind::Arity
                        | crate::analyser::RegistrySourceDiagnosticKind::ArgumentRelationship
                )
            {
                // The sealed source selection owns naming/shadow applicability;
                // a reporting-name lookup cannot change that semantic purpose.
                if subject.kind() != crate::analyser::RegistrySourceDiagnosticKind::Arity
                    || !self.selected_source_package_is_unmentioned(subject)
                {
                    self.result.diagnostics.push(diag);
                }
                continue;
            }
            let call_off = diag.span.start();
            let path = crate::analyser::scope::implicit_command_namespace_path_at(
                &self.result.global_scope,
                call_off,
            );
            if facts.resolves_to_user(&cmd_name, &ns, path, enforce_order, call_off) {
                continue;
            }
            self.result.diagnostics.push(diag);
        }

        // Reborrow the original positioned source-call recipe after the walk.
        // Its declared header and retained alias prefix give conditional source
        // signature advice; report maps never reconstruct selection or argv.
        let user_pending = std::mem::take(&mut self.pending_user_call_arity);
        for cand in user_pending {
            let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
                cand.cmd_name.as_bytes(),
            ))
            .expect("tail of a Unicode command");
            if facts.stub_names.contains(bare) {
                continue;
            }
            let Some(original) = self.original_user_call_signature(cand.call_off) else {
                continue;
            };
            let Some(captured) = u16::try_from(original.captured_argument_count()).ok() else {
                continue;
            };
            let arity = shift_arity(original.declared_arity(), captured);
            let Some(head) = original.original_words().first() else {
                continue;
            };
            let Some(last) = original.original_words().last() else {
                continue;
            };
            let arguments = &original.original_words()[1..];
            let positional_any_expand = arguments.iter().any(|word| word.group().expand);
            let nargs_min = arguments.iter().filter(|word| !word.group().expand).count();
            let arg_spans = if positional_any_expand {
                Vec::new()
            } else {
                arguments
                    .iter()
                    .map(tcl_lexer::NativeWord::span)
                    .collect::<Vec<_>>()
            };
            let full_span = tcl_lexer::Span::new(head.span().start(), last.span().end());
            // The surplus-run anchor: only computable now that the resolved
            // arity's `max` is known. `arg_spans` is empty under `{*}`
            // expansion, so `.get(max)` naturally abstains there.
            let excess = (!arity.is_unlimited())
                .then(|| {
                    let max = usize::from(arity.max);
                    let first = arg_spans.get(max)?;
                    let last = arg_spans.last()?;
                    let delete_from = match max.checked_sub(1).and_then(|i| arg_spans.get(i)) {
                        Some(prev) => prev.end(),
                        None => head.span().end(),
                    };
                    Some(ExcessArgs {
                        span: tcl_lexer::Span::new(first.start(), last.end()),
                        delete_from,
                    })
                })
                .flatten();
            if let Some(diag) = arity_verdict(
                &cand.cmd_name,
                arity,
                nargs_min,
                positional_any_expand,
                full_span,
                excess,
                None,
            ) {
                self.result.diagnostics.push(diag.with_subject(
                    crate::analyser::DiagnosticSubject::UserSourceArity(std::sync::Arc::new(
                        original,
                    )),
                ));
            }
        }
    }

    /// Resolve constructor signature advice through its retained original
    /// class factory, canonical lifecycle declarations and complete source argv.
    pub fn flush_ctor_arity_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_ctor_arity);
        let diagnostics = pending
            .into_iter()
            .filter_map(|candidate| {
                let advice =
                    std::sync::Arc::new(super::types::OriginalConstructorArityAdvice::assess(
                        candidate.original,
                        &self.result,
                        &self.source,
                    )?);
                let count = advice.argument_count();
                arity_verdict(
                    &candidate.display_name,
                    advice.arity(),
                    usize::from(count.minimum),
                    count.indeterminate,
                    advice.call().span()?,
                    None,
                    None,
                )
                .map(|diagnostic| {
                    diagnostic.with_subject(crate::analyser::DiagnosticSubject::ObjectSourceArity(
                        std::sync::Arc::new(
                            crate::analyser::ObjectSourceAritySubject::Constructor(advice),
                        ),
                    ))
                })
            })
            .collect::<Vec<_>>();
        self.result.diagnostics.extend(diagnostics);
    }

    /// Resolve readonly declaration-owned next-chain signature questions.
    /// Current receiver dispatch and source metadata order remain distinct.
    pub fn flush_next_arity_diagnostics(&mut self) {
        let pending = std::mem::take(&mut self.pending_next_arity);
        let diagnostics = pending
            .into_iter()
            .filter_map(|candidate| {
                let arity = candidate
                    .original
                    .source_arity(&self.result, &self.source)?;
                let count = candidate.original.source_argument_count()?;
                let first = candidate.original.original_words().first()?.span();
                let last = candidate.original.original_words().last()?.span();
                arity_verdict(
                    &candidate.display_name,
                    arity,
                    usize::from(count.minimum),
                    count.indeterminate,
                    tcl_lexer::Span::new(first.start(), last.end()),
                    None,
                    None,
                )
                .map(|diagnostic| {
                    diagnostic.with_subject(crate::analyser::DiagnosticSubject::ObjectSourceArity(
                        std::sync::Arc::new(
                            crate::analyser::ObjectSourceAritySubject::LexicalNext(
                                candidate.original,
                            ),
                        ),
                    ))
                })
            })
            .collect::<Vec<_>>();
        self.result.diagnostics.extend(diagnostics);
    }

    /// Queue the `TclOO`-specific arity candidates a call may trigger,
    /// alongside the ordinary same-file/registry checks
    /// [`Self::emit_arity_diagnostics`] always runs: a constructor call
    /// (queued when the first word has a manufacturer descriptor in the
    /// registry) and a next-chain source call selected by its authentic
    /// original lexical member and Registry helper grammar.
    /// Both are resolved post-walk, once `all_classes` is fully
    /// populated — see [`Self::flush_ctor_arity_diagnostics`] /
    /// [`Self::flush_next_arity_diagnostics`]. Split out purely to keep
    /// `emit_arity_diagnostics` under the line-count lint.
    fn queue_tcloo_arity_candidates(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        self.queue_ctor_arity_candidate(cmd_name, cmd_tok, scope_path);
        self.queue_next_arity_candidate(cmd_name, cmd_tok, scope_path);
    }

    /// Record presentation and source position only. Actual call/header/argv
    /// ownership is reborrowed after the walk, including literal sigil names.
    fn queue_user_call_arity_candidate(&mut self, cmd_name: &str, cmd_tok: tcl_lexer::Token) {
        self.pending_user_call_arity.push(PendingUserCallArity {
            cmd_name: cmd_name.to_owned(),
            call_off: cmd_tok.span.start(),
        });
    }

    /// Queue only a retained original class-factory call. Alias captures and
    /// moved names are owned by that producer rather than re-parsed strings.
    fn queue_ctor_arity_candidate(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let Some(original) = super::types::OriginalConstructorArityCall::from_source(
            &self.source,
            &self.result,
            cmd_tok.span.start(),
            !self.scope_path_in_proc_body(scope_path),
        ) else {
            return;
        };
        self.pending_ctor_arity
            .push(super::types::PendingCtorArity {
                original: std::sync::Arc::new(original),
                display_name: cmd_name.to_owned(),
            });
    }

    /// Queue only the complete original Registry helper shape in its genuine
    /// lexical body. Reporting names never select the class or member.
    fn queue_next_arity_candidate(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        scope_path: &[usize],
    ) {
        let Some(context) = self.current_method_context(scope_path) else {
            return;
        };
        let Some(original) = super::types::OriginalLexicalNextCall::from_source(
            context,
            &self.result,
            &self.source,
            &cmd_tok,
        ) else {
            return;
        };
        self.pending_next_arity
            .push(super::types::PendingNextArity {
                original: std::sync::Arc::new(original),
                display_name: cmd_name.to_owned(),
            });
    }

    fn original_user_call_signature(
        &self,
        offset: u32,
    ) -> Option<crate::analyser::SourceUserCallAritySubject> {
        crate::analyser::SourceUserCallAritySubject::capture(&self.source, &self.result, offset)
    }

    /// **E004.** Shape advice from the genuinely selected clause grammar over
    /// structured argv. Unknown payloads keep their positions, while unknown
    /// selector words cannot select a shape. The typed subject owns the
    /// offending written operand; captures cannot borrow its span. Fixes
    /// require a complete consecutive vector of ordinary original words.
    pub(in crate::analyser) fn emit_e004_clause_shape_diagnostic(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use tcl_registry::ClauseShapeError;
        let Some(original) = original else { return };
        let Some(issue) = original
            .with_schema(super::super::diagnostic_registry::source_clause_issue)
            .flatten()
        else {
            return;
        };
        let error = issue.error();
        let argument = match error {
            ClauseShapeError::MissingExpr { after } => after,
            ClauseShapeError::MissingBody { after } => Some(after),
            ClauseShapeError::ExtraWords { first_extra } => Some(first_extra),
        };
        let Some(subject) = original.subject(
            super::super::RegistrySourceDiagnosticKind::ClauseShape,
            argument,
        ) else {
            return;
        };
        let mut span = argument.map_or(original.head().span(), |index| {
            original.word(index).unwrap().span()
        });
        let label = |index| {
            original
                .literal(index)
                .or_else(|| original.word(index)?.try_text().ok())
                .unwrap_or("argument")
        };
        let message = match error {
            ClauseShapeError::MissingExpr { after: None } => {
                format!("No expression after \"{}\" argument", original.command())
            }
            ClauseShapeError::MissingExpr { after: Some(index) } => {
                format!("No expression after \"{}\" argument", label(index))
            }
            ClauseShapeError::MissingBody { after } => {
                format!("No script following \"{}\" argument", label(after))
            }
            ClauseShapeError::ExtraWords { .. } => {
                if let Some(last) = original
                    .words()
                    .arguments()
                    .len()
                    .checked_sub(1)
                    .and_then(|index| original.word(index))
                {
                    span = tcl_lexer::Span::new(span.start(), last.span().end());
                }
                "Extra words after \"else\" clause in \"if\" command".to_owned()
            }
        };
        // Edits require an entirely written, ordinary and consecutive vector.
        // Captured prefixes and expansion children never borrow editable words.
        let written_spans = (0..original.words().arguments().len())
            .map(|index| {
                (original.written_index(index)? == index).then_some(())?;
                Some(original.word(index)?.span())
            })
            .collect::<Option<Vec<_>>>();
        let fixes =
            written_spans.map_or_else(Vec::new, |spans| self.e004_fixes(&spans, issue.repair()));
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::E004, span, message, Severity::Error)
                .with_subject(subject)
                .with_fixes(fixes),
        );
    }

    /// Original selected lexical-context advice. The event flag is source
    /// context; this method grants no native event or procedure frame.
    pub(in crate::analyser) fn emit_w142_context_gate(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        let Some(message) = original
            .with_schema(|schema| schema.authored_source_context_gate(self.current_event.is_some()))
            .flatten()
        else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::RegistrySourceDiagnosticKind::ContextGate,
            None,
        ) else {
            return;
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W142,
                original.head().span(),
                message.to_owned(),
                Severity::Warning,
            )
            .with_subject(subject),
        );
    }

    /// Proposals use anchors authored by the same selected clause grammar and
    /// complete original word spans. No condition spelling selects an anchor.
    fn e004_fixes(
        &self,
        argument_spans: &[tcl_lexer::Span],
        repair: Option<tcl_registry::ClauseShapeRepair>,
    ) -> Vec<super::types::CodeFix> {
        use tcl_registry::ClauseShapeRepair;
        let Some(repair) = repair else {
            return Vec::new();
        };
        let start_index = match repair {
            ClauseShapeRepair::MergeTrailingWords { body } => body,
            ClauseShapeRepair::RemoveTrailingClause { keyword } => keyword,
        };
        let (Some(first), Some(last)) = (argument_spans.get(start_index), argument_spans.last())
        else {
            return Vec::new();
        };
        let span = tcl_lexer::Span::new(first.start(), last.end());
        let (new_text, description) = match repair {
            ClauseShapeRepair::MergeTrailingWords { .. } => {
                let Some(slice) = self.source.get(span.as_range()) else {
                    return Vec::new();
                };
                (
                    format!("{{{slice}}}"),
                    "Merge trailing words into the if body",
                )
            }
            ClauseShapeRepair::RemoveTrailingClause { .. } => {
                (String::new(), "Remove incomplete trailing clause")
            }
        };
        vec![super::types::CodeFix {
            span,
            new_text,
            description: description.to_owned(),
            // Merging or deleting written code requires choosing its intended meaning.
            safety: crate::irules_checks::FixSafety::RequiresReview,
        }]
    }

    /// **W304.** Original source advice before the first data-or-option
    /// boundary in a selected vocabulary that admits `--`. Unknown values
    /// remain conditional; a preceding textual assignment supplies no current
    /// cell value or severity promotion.
    pub(in crate::analyser) fn emit_w304_missing_option_terminator(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
        command: &str,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(scan) = original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        if !scan.accepts_terminator {
            return;
        }
        let (argument, dynamic) = match scan.boundary {
            tcl_registry::AuthoredSourceOptionBoundary::Dynamic(index) => (index, true),
            tcl_registry::AuthoredSourceOptionBoundary::Unknown(index) => (index, false),
            _ => return,
        };
        let Some(word) = original.word(argument) else {
            return;
        };
        let Some(subject) = original.subject(
            super::super::RegistrySourceDiagnosticKind::OptionTerminator,
            Some(argument),
        ) else {
            return;
        };
        let source_extent = word.span();
        let suffix = scan.subcommands.join(" ");
        let label = if suffix.is_empty() {
            command.to_owned()
        } else {
            format!("{command} {suffix}")
        };
        let (severity, message) = if dynamic {
            (
                Severity::Suggestion,
                format!(
                    "'{label}' parses leading '-' as options. Insert '--' before substituted input if it is intended as data."
                ),
            )
        } else {
            (
                Severity::Warning,
                format!(
                    "'{label}' argument starts with '-'. Add '--' before this value if it is intended as data."
                ),
            )
        };
        let fixes = original
            .written_index(argument)
            .and_then(|_| word.try_text().ok())
            .map(|spelling| {
                vec![super::types::CodeFix {
                    span: source_extent,
                    new_text: format!("-- {spelling}"),
                    description: "Insert '--' option terminator".to_owned(),
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                }]
            })
            .unwrap_or_default();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W304,
                source_extent,
                message,
                severity,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }

    /// W217: the selected dialect option protocol consumed every unset word.
    /// A captured option can explain the source shape, but cannot acquire a
    /// written operand or an insertion that reclassifies the captured prefix.
    pub(in crate::analyser) fn emit_w217_unset_option_only(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_unset_option_only_arguments,
        };
        let Some(original) = original else { return };
        let Some(arguments) = original
            .with_schema(source_unset_option_only_arguments)
            .flatten()
        else {
            return;
        };
        let Some(last) = arguments.end.checked_sub(1) else {
            return;
        };
        let subject = original
            .subject_range(Kind::OptionOnly, arguments.start..=last)
            .or_else(|| original.subject(Kind::OptionOnly, None));
        let Some(subject) = subject else { return };
        let span = match &subject {
            crate::analyser::DiagnosticSubject::RegistrySource(subject) => subject.span(),
            _ => return,
        };
        let fixes = original
            .word(arguments.start)
            .filter(|_| original.written_index(arguments.start) == Some(0))
            .and_then(|word| Some((word.span(), word.try_text().ok()?)))
            .map(|(span, spelling)| {
                vec![super::types::CodeFix {
                    span,
                    new_text: format!("-- {spelling}"),
                    description: "Insert '--' so the following words are variable names".to_owned(),
                    safety: crate::irules_checks::FixSafety::BehaviourHardening,
                }]
            })
            .unwrap_or_default();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W217, span,
                "`unset` unsets no variable here — `-nocomplain` / `--` are consumed as options. To unset a variable whose name begins with `-`, put `--` before it (e.g. `unset -- -nocomplain`).".to_owned(),
                Severity::Warning,
            ).with_fixes(fixes).with_subject(subject),
        );
    }

    /// Clear compatibility buffers; source-owned W304 advice is emitted at
    /// its retained original invocation without whole-file value inference.
    pub(in crate::analyser) fn flush_w304_diagnostics(&mut self) {
        self.pending_w304.clear();
    }

    /// **W116 / W117.** Stub command / expression definition shadows a
    /// built-in. W116 uses this analysis generation's selected descriptor;
    /// W117 fires when a stub expr function/operator name
    /// collides with a built-in `expr` function or operator.
    pub(in crate::analyser) fn emit_w116_w117_stub_shadows(&mut self) {
        use super::types::Severity;

        if self.result.stub_commands.is_empty() && self.result.stub_expr_defs.is_empty() {
            return;
        }

        if !self.result.stub_commands.is_empty() {
            let context = self.analysis_context();
            let hits: Vec<(String, tcl_lexer::Span)> = self
                .result
                .stub_commands
                .iter()
                .filter(|stub| {
                    !stub.from_sidecar
                        && context
                            .context()
                            .resolve_spec(context.commands(), &stub.name)
                            .is_some()
                })
                .map(|s| (s.name.clone(), s.range))
                .collect();
            for (name, span) in hits {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W116,
                        span,
                        format!("Stub command '{name}' shadows built-in command."),
                        Severity::Warning,
                    ));
            }
        }

        // W117 — stub expr function/operator shadows a built-in.
        if !self.result.stub_expr_defs.is_empty() {
            let irules = self.profile.is_irules();
            let hits: Vec<(String, String, tcl_lexer::Span)> = self
                .result
                .stub_expr_defs
                .iter()
                .filter(|s| {
                    !s.from_sidecar
                        && (is_builtin_math_function(&s.name)
                            || is_builtin_expr_op(&s.name)
                            || (irules && is_irules_only_expr_op(&s.name)))
                })
                .map(|s| (s.name.clone(), s.kind.clone(), s.range))
                .collect();
            for (name, kind, span) in hits {
                let kind_label = if kind == "function" {
                    "function"
                } else {
                    "operator"
                };
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W117,
                        span,
                        format!(
                            "Stub expression {kind_label} '{name}' shadows built-in {kind_label}."
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// IRULE2002 source deprecation advice from the selected original descriptor.
    /// Metadata can propose a drop-in spelling; source ownership alone makes
    /// the edit a review proposal, without entered replacement-handler proof.
    pub(in crate::analyser) fn emit_irule2002_deprecated_command(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_descriptors,
        };
        let Some(original) = original else {
            return;
        };
        if original
            .context()
            .context()
            .authoring_query()
            .core
            .nearest()
            .is_none_or(|(family, _)| family != tcl_dialect::model::Family::F5Irules)
        {
            return;
        }
        let Some(descriptors) = original.with_schema(source_descriptors) else {
            return;
        };
        let Some(replacement) = descriptors.command.deprecated_replacement else {
            return;
        };
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::DeprecatedCommand, None)
        else {
            return;
        };
        let span = original.head().span();
        let fixes = descriptors
            .command
            .deprecated_replacement_drop_in
            .then(|| self.deprecated_command_source_proposal(original, replacement))
            .flatten()
            .map(|new_text| super::types::CodeFix {
                span,
                new_text,
                description: format!("Replace with '{replacement}'"),
                safety: crate::irules_checks::FixSafety::RequiresReview,
            })
            .into_iter()
            .collect();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::Irule2002,
                span,
                format!(
                    "'{}' is deprecated in iRules. Use '{replacement}' instead.",
                    original.command()
                ),
                Severity::Warning,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }

    fn deprecated_command_source_proposal(
        &self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        replacement: &str,
    ) -> Option<String> {
        if !original.matches_analysis(&self.result, &self.source) {
            return None;
        }
        let head = original.head();
        let config = head.config();
        let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(replacement, config)?;
        let mut proposed = replacement.clone();
        for ordinal in 0..original.words().arguments().len() {
            if original.written_index(ordinal) != Some(ordinal) {
                return None;
            }
            let word = original.word(ordinal)?;
            if word.image() != head.image() || word.config() != config || word.group().expand {
                return None;
            }
            let text =
                std::str::from_utf8(word.image().bytes().get(word.span().as_range())?).ok()?;
            proposed.push(' ');
            proposed.push_str(text);
        }
        let image = tcl_lexer::SourceImage::document(&proposed);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(proposed.len()).ok()?),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        original.words().with_stock_source_command_proposal(
            original.context(),
            &command.words,
            |_| (),
        )?;
        Some(replacement)
    }

    /// W143 is selected Registry source advice where a genuine schema exists.
    /// Otherwise the same Registry-owned namespace policy reads the original
    /// static whole head, retaining explicitly unavailable lookup/handler
    /// applicability. Unregistered helpers cannot acquire a fake descriptor.
    /// Bare direct written heads alone retain the existing review-only public
    /// spelling edit; aliases/captures and delimited heads keep advice only.
    pub(in crate::analyser) fn emit_w143_private_tcl_namespace(
        &mut self,
        cmd_name: &str,
        cmd_tok: tcl_lexer::Token,
        _scope_path: &[usize],
    ) {
        let Some(input) = self.result.resolved_input.clone() else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(&self.source);
        let Some((head, site)) = self
            .head_identities
            .original_written_head_at_span_in_source(&image, cmd_tok.span, input.lexer_config())
        else {
            return;
        };
        if head.group().expand {
            return;
        }
        let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(&head) else {
            return;
        };
        let Ok(literal_head) = std::str::from_utf8(&value) else {
            return;
        };
        let Some((segment, _)) = self
            .head_identities
            .invocation_at_source("", site.offset)
            .original_recorded_command()
        else {
            return;
        };
        let original = self.original_diagnostic_source_for_segment(&segment);
        let selected = original.as_ref().and_then(|source| source.registry());
        // An existing selected descriptor owns registered spelling/aliases.
        // Without one this remains the separate literal-head namespace policy,
        // required for unregistered Whole-namespace implementation helpers.
        let classified_name = selected
            .map_or(literal_head, |selected| selected.command())
            .to_owned();
        let context = input.context_registry();
        let Some(call) = tcl_registry::private_tcl_namespaces::classify_private_tcl_namespace_call(
            &classified_name,
            context.commands(),
            Some(context.context().authoring_query()),
        ) else {
            return;
        };
        let message = match &call.suggestion {
            Some(suggestion) => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; use the public ensemble command instead — e.g. '{suggestion}'."
            ),
            None => format!(
                "'{cmd_name}' is a private Tcl implementation namespace; '{}' is not a subcommand of the public '{}' ensemble, so there is no supported spelling of this call.",
                call.tail, call.public_command
            ),
        };
        let direct_written = selected.is_none_or(|selected| {
            selected.head() == &head
                && (0..selected.words().arguments().len())
                    .all(|argument| selected.written_index(argument) == Some(argument))
        });
        let fixes = call
            .suggestion
            .as_ref()
            .and_then(|suggestion| {
                (direct_written
                    && !head.group().expand
                    && head.group().kind == tcl_lexer::WordKind::Bare)
                    .then_some(())?;
                let public = tcl_syntax::word_rules::lexical_unicode_source_word(
                    call.public_command,
                    head.config(),
                )?;
                let tail =
                    tcl_syntax::word_rules::lexical_unicode_source_word(&call.tail, head.config())?;
                Some(super::types::CodeFix {
                    span: head.span(),
                    new_text: format!("{public} {tail}"),
                    description: format!("Replace with '{suggestion}'"),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                })
            })
            .into_iter()
            .collect();
        let subject = std::sync::Arc::new(
            crate::analyser::SourcePrivateTclNamespaceSubject::from_original_source(
                head,
                input,
                site.offset,
                selected.map(|selected| selected.words().clone()),
                classified_name,
                call,
            ),
        );
        let diagnostic =
            super::types::Diagnostic::new(DiagCode::W143, cmd_tok.span, message, Severity::Warning)
                .with_fixes(fixes)
                .with_subject(crate::analyser::DiagnosticSubject::PrivateTclNamespace(
                    subject.clone(),
                ));
        self.pending_w143.push(PendingPrivateTclSourceAdvice {
            subject,
            diagnostic,
        });
    }

    /// Whole-file source ownership suppressions are deliberately order-independent
    /// dependency advice. Every request offset reissues a genuine current
    /// selected Require operand; reporting package/name maps are not authority.
    /// Source calls/classes similarly use their authentic declaration receipts.
    /// Drains the original pending source-policy receipts once.
    pub(in crate::analyser) fn flush_w143_diagnostics(&mut self) {
        if self.pending_w143.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_w143);
        for candidate in pending {
            if !candidate.subject.matches_source(&self.source, &self.result) {
                continue;
            }
            let Some(input) = self.result.resolved_input.as_ref() else {
                continue;
            };
            let image = candidate.subject.original_head().image();
            let offset = candidate.subject.source_offset();
            // Genuine source declarations are positive suppression advice;
            // report QName/alias maps do not manufacture a call target.
            let declared = if input.has_logical_source_name_context() {
                crate::registry_invocation::source_structure::source_logical_procedure_call_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            } else {
                crate::registry_invocation::source_structure::source_original_procedure_call_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            };
            if declared
                || crate::registry_invocation::source_structure::source_class_instance_words_at(
                    &self.source,
                    &self.result,
                    offset,
                )
                .is_some()
            {
                continue;
            }
            let context = input.context_registry();
            let package_covers = self.result.package_requires.iter().any(|required| {
                let Some((_, offset)) = self
                    .head_identities
                    .original_written_word_at_span_in_source(
                        image,
                        required.range,
                        input.lexer_config(),
                    )
                else {
                    return false;
                };
                let Some(words) =
                    crate::registry_invocation::source_structure::source_registry_words_at(
                        &self.source,
                        &self.result,
                        offset,
                    )
                else {
                    return false;
                };
                let Some(reference) = words.package_reference(&context) else {
                    return false;
                };
                if reference.kind()
                    != tcl_registry::source_navigation::SourcePackageReferenceKind::Require
                {
                    return false;
                }
                let Some(word) = words.original_argument_word(reference.argument()) else {
                    return false;
                };
                let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                else {
                    return false;
                };
                let Ok(package) = std::str::from_utf8(&value) else {
                    return false;
                };
                tcl_registry::private_tcl_namespaces::required_package_namespace_advice_covers(
                    package,
                    candidate.subject.classified_source_name(),
                )
            });
            if !package_covers {
                self.result.diagnostics.push(candidate.diagnostic);
            }
        }
    }

    /// Typed Registry source shape advice for the selected deprecated command.
    /// Complete original word spellings preserve braces, quotes, substitutions,
    /// escapes and newlines; captured or expanded argument layouts get no edit.
    pub(in crate::analyser) fn emit_source_deprecation_advice(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind, source_descriptors,
        };
        let Some(original) = original else {
            return;
        };
        let Some(descriptors) = original.with_schema(source_descriptors) else {
            return;
        };
        let Some(advice) = descriptors.command.source_deprecation_advice else {
            return;
        };
        if !advice.applies_to(original.context().context().authoring_query()) {
            return;
        }
        let Some(subject) = original.subject(RegistrySourceDiagnosticKind::DeprecatedCommand, None)
        else {
            return;
        };
        let span = original.head().span();
        let count = original.words().arguments().len();
        let raw = (0..count)
            .map(|index| {
                (original.written_index(index) == Some(index)).then_some(())?;
                original.word(index)?.try_text().ok()
            })
            .collect::<Option<Vec<_>>>();
        let fixes = raw
            .as_deref()
            .and_then(|arguments| advice.replacement(arguments))
            .and_then(|new_text| {
                let last = original.word(count.checked_sub(1)?)?;
                Some(vec![super::types::CodeFix {
                    span: tcl_lexer::Span::new(span.start(), last.span().end()),
                    new_text,
                    description: advice.description().to_owned(),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }])
            })
            .unwrap_or_default();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                advice.code(),
                span,
                advice.message().to_owned(),
                Severity::Warning,
            )
            .with_fixes(fixes)
            .with_subject(subject),
        );
    }

    /// **W004.** Explain excluded exact option rows from the same retained
    /// source descriptor, vocabulary and value-width owner as lifecycle advice.
    /// Values and reserved operands never become new option candidates.
    pub(in crate::analyser) fn emit_w004_dialect_invalid_option(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
        command: &str,
    ) {
        let Some(original) = original else {
            return;
        };
        let Some(scan) = original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
        else {
            return;
        };
        let subcommand = (!scan.subcommands.is_empty()).then(|| scan.subcommands.join(" "));
        for selected in scan
            .options
            .into_iter()
            .filter(|selected| !selected.available)
        {
            let Some(word) = original.word(selected.argument) else {
                continue;
            };
            let Some(spelling) = original.literal(selected.argument) else {
                continue;
            };
            let Some(subject) = original.subject(
                super::super::RegistrySourceDiagnosticKind::DisabledOption,
                Some(selected.argument),
            ) else {
                continue;
            };
            let suffix = subcommand
                .as_deref()
                .map_or(String::new(), |sub| format!(" {sub}"));
            let fixes = Self::original_option_removal_fix(original, &selected, spelling);
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                DiagCode::W004, word.span(),
                format!("Option '{spelling}' on '{command}'{suffix} is not available in the active dialect ({}).", self.dialect()),
                Severity::Warning,
            ).with_fixes(fixes).with_subject(subject));
        }
        if let tcl_registry::AuthoredSourceOptionBoundary::Ambiguous {
            argument,
            candidates,
        } = scan.boundary
        {
            let Some(spelling) = original.literal(argument) else {
                return;
            };
            let Some(subject) = original.subject(
                super::super::RegistrySourceDiagnosticKind::DisabledOption,
                Some(argument),
            ) else {
                return;
            };
            let before = self.result.diagnostics.len();
            self.emit_w145_ambiguous_option(
                command,
                subcommand.as_deref(),
                spelling,
                &candidates
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect::<Vec<_>>(),
                original,
                argument,
            );
            if let Some(diagnostic) = self.result.diagnostics.get_mut(before) {
                diagnostic.subject = Some(subject);
            }
        }
    }

    fn original_option_removal_fix(
        original: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        selected: &tcl_registry::AuthoredSourceOption<'_>,
        spelling: &str,
    ) -> Vec<super::types::CodeFix> {
        let Some(values) = &selected.values else {
            return Vec::new();
        };
        let Some(first) = original.word(selected.argument) else {
            return Vec::new();
        };
        let last = if values.is_empty() {
            Some(first)
        } else {
            original.word(values.end - 1)
        };
        let Some(last) = last else {
            return Vec::new();
        };
        let Some(written) = original.written_index(selected.argument) else {
            return Vec::new();
        };
        if !(selected.argument..values.end).all(|argument| {
            original.written_index(argument) == Some(written + argument - selected.argument)
        }) {
            return Vec::new();
        }
        let end = original
            .word(values.end)
            .filter(|next| {
                next.span().start() >= last.span().end()
                    && original.written_index(values.end)
                        == Some(written + values.end - selected.argument)
            })
            .map_or(last.span().end(), |next| next.span().start());
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(first.span().start(), end),
            new_text: String::new(),
            description: format!("Remove '{spelling}' option"),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        }]
    }

    /// **W145** at an option word. Same contract as the subcommand form:
    /// name the matching candidates and offer one manual-pick fix each.
    fn emit_w145_ambiguous_option(
        &mut self,
        cmd_name: &str,
        sub_name: Option<&str>,
        word: &str,
        candidates: &[String],
        original: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        argument: usize,
    ) {
        let Some(operand) = original.word(argument) else {
            return;
        };
        let span = operand.span();
        let sub_suffix = sub_name.map_or(String::new(), |n| format!(" {n}"));
        let listed = candidates
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let fixes: Vec<super::types::CodeFix> = candidates
            .iter()
            .filter_map(|candidate| {
                original.written_index(argument)?;
                let new_text = tcl_syntax::word_rules::lexical_unicode_source_word(
                    candidate,
                    operand.config(),
                )?;
                Some(super::types::CodeFix {
                    span,
                    new_text,
                    description: format!("Expand to '{candidate}'"),
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                })
            })
            .collect();
        self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W145,
    span,
    format!(
                "Ambiguous abbreviation '{word}' for '{cmd_name}{sub_suffix}': matches {listed}."
            ),
    Severity::Warning,
).with_fixes(fixes));
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warnings for an EXPR-role argument that uses a Tcl
    /// 9.0 string-comparison operator (`lt` / `le` / `gt` / `ge`,
    /// TIP 461) in a pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl
    /// 8.5+) in an earlier one. One diagnostic is emitted per
    /// offending operator *occurrence*, anchored at that operator's
    /// own token span rather than the whole argument, so the
    /// squiggle stays tight around the actual problem even inside a
    /// large compound expression.
    ///
    /// `content_span` is the argument's inner content: the caller has
    /// already stripped any wrapping `{}` / `""` / `$` via the
    /// token's `content_offset`, so this slices `self.source`
    /// directly instead of trusting a possibly-reconstructed text —
    /// a quoted `"$x lt $y"` argument's word text is canonicalised to
    /// `"${x} lt ${y}"` by the segmenter, which would misalign any
    /// offset computed from it.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        content_span: tcl_lexer::Span,
    ) {
        let start = content_span.start() as usize;
        let end = content_span.end() as usize;
        let Some(expr_text) = Analyser::source_slice(&self.source, start, end) else {
            return;
        };
        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(original) = self.original_expression_parser_context() else {
            return;
        };
        let Some(base) = original.expr_grammar_base else {
            return;
        };
        let f5_words = original.f5_word_grammar.is_some();

        // Operator assistance recognises the full shipped vocabulary while
        // preserving this document's lexical rules. Availability remains the
        // actual profile's gate above; this tree carries no native proof.
        let trimmed = expr_text.trim();
        let assistance = gated_operator_assistance_context(original);
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(trimmed, &assistance);
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }

        // Re-tokenise the same (trimmed) text at the flat lexical
        // level: every gated-keyword `Operator` token here
        // corresponds 1:1 to a `Binary` node the parse above just
        // confirmed is real (a successful parse consumes the whole
        // token stream, and `in`/`ni`/`lt`/`le`/`gt`/`ge`/the iRules
        // words are not valid prefix/atom tokens — the only way one is
        // consumed is as a genuine infix application). This gives an
        // exact per-occurrence span directly from `ExprToken::start/end`
        // without adding source-position fields to `ExprNode::Binary`
        // (which recursive/optimiser/codegen consumers across the
        // compiler pattern-match on by name, not span).
        let (tokens, _) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
            trimmed,
            &assistance.lexer_grammar,
            assistance.expr_grammar_base,
            assistance.f5_word_grammar,
        );
        let gated: Vec<(&tcl_lexer::ExprToken, &'static str)> = tokens
            .iter()
            .filter(|t| t.kind == tcl_lexer::ExprTokenType::Operator)
            .filter_map(|t| gated_operator_name(&t.text, base, f5_words).map(|name| (t, name)))
            .collect();
        if gated.is_empty() {
            return;
        }

        // A mechanical rewrite is only safe to offer when the whole
        // argument IS exactly the one gated application (`2 in {1 2
        // 3}`, `$x lt $y`) with plain operands: the operator nested
        // inside a larger expression, more than one gated occurrence,
        // or an operand that isn't a single bare-word-safe atom
        // (`Binary`/`Unary`/`Ternary`/`Call` — e.g. `max($a, $b)`
        // contains an unprotected space) can't be reduced to one
        // local text replacement.
        let fix_text = (gated.len() == 1)
            .then_some(&parsed)
            .and_then(|node| match node {
                ExprNode::Binary { op, left, right }
                    if is_simple_operand(left) && is_simple_operand(right) =>
                {
                    self.gated_operator_source_proposal(
                        content_span,
                        trimmed,
                        &assistance,
                        &parsed,
                        (op.as_str(), left, right),
                    )
                }
                _ => None,
            });

        // Leading whitespace `expr_text.trim()` stripped, needed to
        // translate a `trimmed`-local offset back to an
        // `expr_text`-local (and then absolute source) offset.
        let trim_base = u32::try_from(expr_text.len() - expr_text.trim_start().len()).unwrap_or(0);

        for (tok, op_name) in gated {
            let op_span = tcl_lexer::Span::new(
                content_span.start() + trim_base + tok.start,
                // `ExprToken::end` is inclusive; `Span` is exclusive.
                content_span.start() + trim_base + tok.end + 1,
            );
            let mut fixes = Vec::new();
            if let Some(new_text) = fix_text.clone() {
                fixes.push(super::types::CodeFix {
                    span: tcl_lexer::Span::new(
                        content_span.start() + trim_base,
                        content_span.start()
                            + trim_base
                            + u32::try_from(trimmed.len()).unwrap_or(0),
                    ),
                    new_text,
                    description: format!(
                        "Rewrite to a form supported by dialect '{}'",
                        self.dialect()
                    ),
                    // W003: the older-dialect spelling of an operator is rarely exact —
                    // `lt` compares as strings where `<` coerces numerically.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W003,
    op_span,
    format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
    Severity::Warning,
).with_fixes(fixes));
        }
    }

    fn gated_operator_source_proposal(
        &self,
        content_span: tcl_lexer::Span,
        trimmed: &str,
        assistance: &tcl_syntax::expr::parser::ExprParseContext,
        parsed: &ExprNode,
        (op, left, right): (&str, &ExprNode, &ExprNode),
    ) -> Option<String> {
        let config = self.file_lexer_config();
        let image = tcl_lexer::SourceImage::document(&self.source);
        let (word, offset) = self
            .head_identities
            .original_written_word_at_span_in_source(&image, content_span, config)?;
        if word.group().kind != tcl_lexer::WordKind::Braced
            || word.content_span().ok()? != content_span
            || word.image().bytes().get(content_span.as_range())?
                != self.source.as_bytes().get(content_span.as_range())?
        {
            return None;
        }
        let (command, _) = self
            .head_identities
            .invocation_at_source("", offset)
            .original_recorded_command()?;
        let original = self.original_diagnostic_source_for_segment(&command)?;
        let registry = original.registry()?;
        if !registry.matches_analysis(&self.result, &self.source) {
            return None;
        }
        let roles = registry
            .with_schema(crate::analyser::diagnostic_registry::source_expression_arguments)??;
        let mut matching = roles.arguments.into_iter().filter(|&argument| {
            original.written_index(argument).is_some() && original.word(argument) == Some(&word)
        });
        matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let source = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            trimmed, assistance, config,
        )?;
        if !source.matches_tree(parsed) {
            return None;
        }
        let left = source.proposed_tcl_argument_source(left)?;
        let right = source.proposed_tcl_argument_source(right)?;
        let proposal = rewrite_gated_operator(op, &left, &right)?;
        // The shared expression AST owns the generated bracket's real script
        // region. No prefix stripping or parsed name donates original identity.
        let proposed = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            &proposal, assistance, config,
        )?;
        let regions = proposed.command_regions()?;
        let [region] = regions.as_slice() else {
            return None;
        };
        let body = proposal.get(region.body.as_range())?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(body),
            tcl_lexer::Span::new(0, u32::try_from(body.len()).ok()?),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        registry.words().with_stock_source_command_proposal(
            registry.context(),
            &command.words,
            |_| (),
        )?;
        Some(proposal)
    }

    /// **W002** for a math function used before its introducing release —
    /// `min(…)` under 8.4, an `is*(…)` classification under 8.6.  The function
    /// dispatches to `::tcl::mathfunc::<name>`, a command that simply does not
    /// exist in the older core, so the same disabled-in-dialect diagnostic the
    /// command path emits applies to the function token in the expression.
    pub(in crate::analyser) fn emit_expr_function_dialect_diagnostics(
        &mut self,
        expr_tok: tcl_lexer::Token,
    ) {
        let Some(ceiling) = crate::tcl_expr_eval::math_func_ceiling_for_dialect(self.profile)
        else {
            return;
        };
        for (name, span, _argc) in self.expr_function_calls(expr_tok) {
            let Some(since) = tcl_syntax::expr::mathfunc::added_in(&name) else {
                continue;
            };
            if since > ceiling {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W002,
                        span,
                        format!(
                            "'{}' is disabled in the active dialect profile ('{}')",
                            tcl_registry::mathfunc::qualified_name(&name),
                            self.dialect()
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// **W003** for the multi-word `expr a b c …` form. Tcl's `expr`
    /// is the only EXPR-role command whose expression can be spread
    /// across several unbraced words (`if`/`while`/`for` require
    /// their condition to be a single Tcl word — an unbraced
    /// multi-word condition is a Tcl arity error, not valid syntax).
    /// Written this way, a gated operator keyword is necessarily its
    /// own standalone Tcl word, so its `arg_tokens` entry is already
    /// an exact, tight span — no text-offset remapping needed, unlike
    /// the single-argument path above.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_words(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        joined_text: &str,
    ) {
        if !contains_gated_word(joined_text) {
            return;
        }
        let Some(original) = self.original_expression_parser_context() else {
            return;
        };
        let Some(base) = original.expr_grammar_base else {
            return;
        };
        let f5_words = original.f5_word_grammar.is_some();
        let assistance = gated_operator_assistance_context(original);
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
            joined_text.trim(),
            &assistance,
        );
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        for (word, tok) in args.iter().zip(arg_tokens.iter()) {
            let Some(op_name) = gated_operator_name(word, base, f5_words) else {
                continue;
            };
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W003,
    tok.span,
    format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
    Severity::Warning,
).with_fixes(// The unbraced multi-word form has no single local
                // span to rewrite in place without also re-bracing
                // the whole expression (W100's concern, not this
                // one) — left unfixed.
Vec::new()));
        }
    }
}

/// Whether `name` is a built-in `expr` math function (`sin`, `max`, the TIP
/// 745 C99 batch, …) in *any* dialect — used by the W117 stub-shadow check,
/// which doesn't have (and doesn't need) a specific dialect to check against
/// here since a stub shadowing a function that exists in some other dialect
/// is still worth flagging. Derived from
/// [`tcl_syntax::expr::mathfunc::added_in`] rather than a hand-typed list,
/// which drifts (a list claiming the TIP 745 batch before `dispatch()`
/// implements it).
fn is_builtin_math_function(name: &str) -> bool {
    tcl_syntax::expr::mathfunc::added_in(name).is_some()
}

/// Whether `name` is a built-in `expr` operator spelling **not** gated to
/// iRules only (`+`, `in`, `**`, `eq`, …) — used by the W117 stub-shadow
/// check. Derived from [`tcl_syntax::expr::operators`] rather than a
/// hand-typed list: a `BinOp`/`UnaryOp` whose
/// `dialects` isn't exactly `Some(SpecSurface::IRULES)` is available outside
/// iRules (`None` = ungated, `Some(TCL90_PLUS)` etc. = version-gated but not
/// dialect-*identity*-gated — both count as "built-in" here; only the nine
/// iRules word operators are excluded).
fn is_builtin_expr_op(name: &str) -> bool {
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface != Some(SpecSurface::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface != Some(SpecSurface::IRULES)
    })
}

/// Whether `name` is one of the nine iRules-only word operators (`and`,
/// `contains`, `not`, …) — see [`is_builtin_expr_op`]'s doc for the
/// derivation and why these are excluded there.
fn is_irules_only_expr_op(name: &str) -> bool {
    tcl_syntax::expr::operators::ALL_BIN_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface == Some(SpecSurface::IRULES)
    }) || tcl_syntax::expr::operators::ALL_UNARY_OPS.iter().any(|op| {
        let spec = op.spec();
        spec.spelling == name && spec.surface == Some(SpecSurface::IRULES)
    })
}

/// Recognition context for operator availability assistance, never native
/// syntax acceptance. The warning compares each recognised operator with the
/// actual profile separately; lexical and numeral rules remain source-selected.
fn gated_operator_assistance_context(
    mut context: tcl_syntax::expr::parser::ExprParseContext,
) -> tcl_syntax::expr::parser::ExprParseContext {
    context.expr_grammar_base = Some(tcl_dialect::TclVersion::V9_1);
    context.f5_word_grammar = tcl_dialect::DialectProfile::irules().f5_core_expr_grammar();
    context.native_syntax = tcl_syntax::expr::parser::NativeExprSyntax::Unknown;
    context
}

/// A version-gated `expr` operator: its text, whether it is word-shaped, the
/// minimum Tcl version it needs, and the TIP citation for the W003 message.
///
/// Single source for the three W003 steps — the prefilter
/// ([`contains_gated_word`]), the per-token gate check ([`gated_operator_name`]),
/// and the message ([`w003_tip_citation`]). `op`/`word_shaped`/`min_version`
/// are derived from [`tcl_syntax::expr::operators`] — only the TIP citation
/// number is local prose (two
/// operators can share a minimum version but not a TIP: `**` is TIP 123,
/// `in`/`ni` is TIP 201, both Tcl 8.5). Three separate hardcoded matches
/// drift from each other instead — with the symbolic `**` in none of them,
/// `expr {2 ** 3}` under tcl8.4 is a false negative.
struct GatedExprOp {
    /// The operator text as the expr lexer emits it (`in`, `**`).
    op: &'static str,
    /// Word-shaped (`in`/`lt`) operators need identifier-boundary matching in
    /// the prefilter; symbolic ones (`**`) match on any occurrence.
    word_shaped: bool,
    /// What must hold for this operator to be valid: a minimum `expr`-grammar
    /// version, or iRules dialect identity.
    gate: ExprOpGate,
    /// The TIP citation surfaced in the W003 message.
    tip: &'static str,
}

/// What a [`GatedExprOp`] requires of the active dialect.
#[derive(Clone, Copy)]
enum ExprOpGate {
    /// The dialect's `expr`-grammar base version must be at least this.
    MinVersion(tcl_dialect::TclVersion),
    /// The dialect must *be* iRules (identity, not a version threshold) —
    /// these 9 word operators (`contains`, `and`, …) have no
    /// `::tcl::mathop` form and exist in no other dialect at any version.
    IrulesOnly,
}

/// The TIP citation string for `spelling`'s W003 message — the one fact a
/// gated operator's `tcl_syntax::expr::operators::OperatorSpec` doesn't
/// carry (see [`GatedExprOp`]'s docs for why).
fn w003_tip_string(spelling: &str) -> &'static str {
    match spelling {
        "**" => "Tcl 8.5+ (TIP 123)",
        "in" | "ni" => "Tcl 8.5+ (TIP 201)",
        "and" | "or" | "not" | "contains" | "starts_with" | "ends_with" | "equals"
        | "matches_glob" | "matches_regex" => "the iRules/BIG-IP Tcl dialect",
        // "lt" | "le" | "gt" | "ge", and the fallback `gated_expr_ops()`
        // itself guarantees no other spelling ever reaches this function.
        _ => "Tcl 9.0+ (TIP 461)",
    }
}

/// The dialect-gated `expr` operators: version-gated (`in`/`ni`/`**` from
/// 8.5; the string comparison words `lt`/`le`/`gt`/`ge` from 9.0) — every
/// `BinOp` whose
/// [`tcl_syntax::expr::operators::OperatorSpec::expr_grammar_min_version`]
/// is `Some(_)` — plus the 9 iRules-only word operators, every
/// `BinOp`/`UnaryOp` whose `OperatorSpec::dialects` is exactly
/// `Some(SpecSurface::IRULES)`. Computed once (not `const`: `OperatorSpec`
/// isn't cheaply iterable in a const context) and cached for the process
/// lifetime — W003 only calls into this after its own text prefilter narrows
/// to expressions that already contain a gated keyword, so this never runs
/// on a hot path.
fn gated_expr_ops() -> &'static [GatedExprOp] {
    static TABLE: std::sync::OnceLock<Vec<GatedExprOp>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let version_gated = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                let min_version = spec.expr_grammar_min_version?;
                Some(GatedExprOp {
                    op: spec.spelling,
                    word_shaped: spec.spelling.as_bytes()[0].is_ascii_alphabetic(),
                    gate: ExprOpGate::MinVersion(min_version),
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_bin = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.surface == Some(SpecSurface::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_un = tcl_syntax::expr::operators::ALL_UNARY_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.surface == Some(SpecSurface::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        version_gated.chain(irules_bin).chain(irules_un).collect()
    })
}

/// Return `true` if `text` contains any dialect-gated expression operator (a
/// word-shaped one as a whole word, or a symbolic one such as `**` anywhere).
/// Used as a fast prefilter to skip the expression parse for expressions that
/// obviously can't trigger W003. Word-boundary matching is whitespace-aware:
/// any non-identifier byte (parentheses, operators, …) counts as a boundary,
/// matching Tcl expr's tolerance for arbitrary whitespace between tokens.
pub(super) fn contains_gated_word(text: &str) -> bool {
    let bytes = text.as_bytes();
    for g in gated_expr_ops() {
        let needle = g.op.as_bytes();
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == needle {
                let word_ok = !g.word_shaped
                    || ((i == 0 || !is_ident_continue(bytes[i - 1]))
                        && (i + n == bytes.len() || !is_ident_continue(bytes[i + n])));
                if word_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

/// The gated operator name for `word` under a dialect whose `expr`-grammar
/// base version is `base` and whose F5-family word-operator acceptance is
/// `f5_words` — `None` when `word` isn't one of the dialect-gated keywords,
/// or the active dialect already satisfies its gate (version threshold met,
/// or the dialect is F5Tcl-cored for a word-form operator — a trunk fact,
/// measured valid in tmsh/iApp expr too, measurements §4a).
fn gated_operator_name(
    word: &str,
    base: tcl_dialect::TclVersion,
    f5_words: bool,
) -> Option<&'static str> {
    gated_expr_ops()
        .iter()
        .find(|g| {
            g.op == word
                && match g.gate {
                    ExprOpGate::MinVersion(min) => base < min,
                    ExprOpGate::IrulesOnly => !f5_words,
                }
        })
        .map(|g| g.op)
}

/// The TIP citation to surface in the W003 message for `op_name`.
fn w003_tip_citation(op_name: &str) -> &'static str {
    gated_expr_ops()
        .iter()
        .find(|g| g.op == op_name)
        .map_or("Tcl 9.0+ (TIP 461)", |g| g.tip)
}

/// Whether `node` is safe to splice as a bare Tcl word into a
/// rewritten command (`lsearch` / `string compare`) with no extra
/// quoting: it never contains an unprotected top-level space. Plain
/// atoms are always safe (`Literal` is bare digits, `String` already
/// carries its own `{}`/`""` delimiters, `Var` is `$name`/`${name}`,
/// `Command` is bracket-delimited); `Binary`/`Unary`/`Ternary` would
/// need re-parenthesising and `Call` (`max($a, $b)`) contains an
/// unprotected space after the comma, so none of those are rewritten.
fn is_simple_operand(node: &ExprNode) -> bool {
    matches!(
        node,
        ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::Var { .. }
            | ExprNode::Command { .. }
    )
}

/// Build the portable rewrite for a dialect-gated `expr` operator
/// application, or `None` if `op_name` isn't one of the six gated
/// keywords. `left`/`right` must already be safe bare-word text (see
/// [`is_simple_operand`]) — the caller is responsible for checking
/// that before rendering them.
fn rewrite_gated_operator(op_name: &str, left: &str, right: &str) -> Option<String> {
    Some(match op_name {
        "in" => format!("([lsearch -exact {right} {left}] >= 0)"),
        "ni" => format!("([lsearch -exact {right} {left}] < 0)"),
        "lt" => format!("([string compare {left} {right}] < 0)"),
        "le" => format!("([string compare {left} {right}] <= 0)"),
        "gt" => format!("([string compare {left} {right}] > 0)"),
        "ge" => format!("([string compare {left} {right}] >= 0)"),
        _ => return None,
    })
}

#[cfg(test)]
mod w117_tests {
    use super::{is_builtin_expr_op, is_builtin_math_function, is_irules_only_expr_op};

    /// The W117 stub-shadow check's `is_builtin_*` helpers are derived from
    /// `tcl_syntax`; these lists are the hand-typed sets they replace
    /// (`BUILTIN_MATH_FUNCTIONS` / `BUILTIN_EXPR_OPS` / `IRULES_EXPR_OPS`),
    /// kept here to prove the derived helpers recognise exactly the same
    /// names.
    const OLD_BUILTIN_MATH_FUNCTIONS: &[&str] = &[
        "abs",
        "acos",
        "asin",
        "atan",
        "atan2",
        "bool",
        "ceil",
        "cos",
        "cosh",
        "double",
        "entier",
        "exp",
        "floor",
        "fmod",
        "hypot",
        "int",
        "isinf",
        "isnan",
        "isqrt",
        "log",
        "log10",
        "max",
        "min",
        "pow",
        "rand",
        "round",
        "sin",
        "sinh",
        "sqrt",
        "srand",
        "tan",
        "tanh",
        "wide",
        "acosh",
        "asinh",
        "atanh",
        "cbrt",
        "copysign",
        "dim",
        "erf",
        "erfc",
        "exp2",
        "expm1",
        "fma",
        "gamma",
        "ldexp",
        "lgamma",
        "log1p",
        "log2",
        "logb",
        "nextafter",
        "remainder",
        "signbit",
        "trunc",
    ];
    const OLD_BUILTIN_EXPR_OPS: &[&str] = &[
        "!", "!=", "%", "&", "&&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=",
        ">>", "^", "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "||", "~",
    ];
    const OLD_IRULES_EXPR_OPS: &[&str] = &[
        "and",
        "contains",
        "ends_with",
        "equals",
        "matches_glob",
        "matches_regex",
        "not",
        "or",
        "starts_with",
    ];

    #[test]
    fn is_builtin_math_function_matches_the_old_hand_list_exactly() {
        for name in OLD_BUILTIN_MATH_FUNCTIONS {
            assert!(
                is_builtin_math_function(name),
                "{name}: was in the old list"
            );
        }
        // A handful of names that were never math functions.
        for name in ["puts", "expr", "+", "in", "notafunction"] {
            assert!(
                !is_builtin_math_function(name),
                "{name}: not a math function"
            );
        }
    }

    #[test]
    fn is_builtin_expr_op_matches_the_old_hand_list_exactly() {
        for name in OLD_BUILTIN_EXPR_OPS {
            assert!(is_builtin_expr_op(name), "{name}: was in the old list");
        }
        for name in OLD_IRULES_EXPR_OPS {
            assert!(
                !is_builtin_expr_op(name),
                "{name}: is iRules-only, must not count as a plain built-in"
            );
        }
        assert!(!is_builtin_expr_op("notanoperator"));
    }

    #[test]
    fn is_irules_only_expr_op_matches_the_old_hand_list_exactly() {
        for name in OLD_IRULES_EXPR_OPS {
            assert!(is_irules_only_expr_op(name), "{name}: was in the old list");
        }
        for name in OLD_BUILTIN_EXPR_OPS {
            assert!(
                !is_irules_only_expr_op(name),
                "{name}: is a plain built-in, must not count as iRules-only"
            );
        }
        assert!(!is_irules_only_expr_op("notanoperator"));
    }
}

#[cfg(test)]
mod stub_name_geometry_tests {
    use super::UserResolutionFacts;

    #[test]
    fn document_stub_suppression_uses_written_tail_geometry() {
        let facts = UserResolutionFacts {
            non_proc_qnames: rustc_hash::FxHashSet::default(),
            proc_offsets: rustc_hash::FxHashMap::default(),
            rename_offsets: rustc_hash::FxHashMap::default(),
            stub_names: ["known", "a:b"].into_iter().map(str::to_owned).collect(),
        };
        for name in ["known", "::ns:::known", "::ns::::known", "::ns::a:b"] {
            assert!(
                facts.resolves_to_user(name, "::other", &[], true, 0),
                "{name}"
            );
        }
        for name in ["::ns::other", "::ns::", "::ns::a::b"] {
            assert!(
                !facts.resolves_to_user(name, "::other", &[], true, 0),
                "{name}"
            );
        }
    }
}

#[cfg(test)]
mod selected_formal_tests {
    use crate::analyser::Analyser;
    use tcl_core_types::DiagCode;

    #[test]
    fn original_formal_diagnostics_use_selected_name_grammar_and_whole_words() {
        // naming.variable.original-readonly-formal-topology
        // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
        // Source consumer contract; these assertions do not enter a native frame.
        for name in ["a::b", "a(1)"] {
            let source = format!("proc target {{{name}}} {{}}");
            for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
                let analysis = Analyser::new().analyse(&source, dialect);
                let diagnostic = analysis
                    .diagnostics
                    .iter()
                    .find(|d| d.code == DiagCode::E006)
                    .expect(dialect);
                assert_eq!(
                    &source[diagnostic.span.start() as usize..diagnostic.span.end() as usize],
                    format!("{{{name}}}")
                );
            }
            let jim = Analyser::new().analyse(&source, "jimtcl");
            assert!(
                !jim.diagnostics.iter().any(|d| d.code == DiagCode::E006),
                "Jim source name grammar: {name}: {:?}",
                jim.diagnostics
            );
        }
    }

    #[test]
    fn original_formal_diagnostics_reject_three_fields_and_withdraw_known_shadows() {
        // naming.variable.original-readonly-formal-topology
        // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
        let malformed = "proc target {{a b c}} {}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let analysis = Analyser::new().analyse(malformed, dialect);
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .any(|d| d.code == DiagCode::E006),
                "selected malformed formal: {dialect}"
            );
        }
        for source in [
            "rename proc define; define target {{a b c}} {}",
            "interp alias {} define {} proc target; define {{a b c}} {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .any(|d| d.code == DiagCode::E006),
                "effective original formal role: {source}"
            );
        }
        let source = "proc proc args {}
proc target {{a b c}} {}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            !analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E006),
            "a known replacement has no original formal-list role: {:?}",
            analysis.diagnostics
        );
    }
}

#[cfg(test)]
mod logical_class_family_tests {
    use super::*;
    use crate::analyser::types::{MetaclassProvenance, SourceNameAmbiguity};
    use std::sync::Arc;

    const SOURCE: &str = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";

    fn inspected_analysis(dialect: &str) -> Analyser {
        let mut analyser = Analyser::new();
        let result = analyser.analyse(SOURCE, dialect);
        analyser.context = result
            .resolved_input
            .as_ref()
            .map(crate::analyser::ResolvedAnalysisInput::context_registry);
        analyser.resolved_input = result.resolved_input.clone();
        analyser.source = SOURCE.to_owned();
        analyser.result = result;
        analyser
    }

    fn family_is_tcloo(analyser: &Analyser) -> bool {
        analyser
            .result
            .all_classes
            .get("::Dog")
            .is_some_and(|class| is_tcloo_source_class(analyser, class))
    }

    #[test]
    fn logical_class_family_uses_observed_definer_without_borrowing_native_formals() {
        // naming.diagnostics.retained-logical-class-family
        // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
        let analyser = inspected_analysis("tcl");
        assert!(analyser.result.allows_retained_logical_declaration_advice());
        let class = analyser.result.all_classes.get("::Dog").unwrap();
        assert_eq!(class.metaclass_provenance, MetaclassProvenance::Observed);
        assert!(class.source_name.is_none());
        assert!(family_is_tcloo(&analyser));
        let dialect = tcl_registry::InvocationDialect::of_profile(
            analyser
                .result
                .resolved_input
                .as_ref()
                .unwrap()
                .unit_profile(),
        );
        assert!(dialect.native_name_protocol().is_none());
        assert!(dialect.parameter_grammar().is_none());
        assert!(
            analyser
                .result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E001)
        );
    }

    #[test]
    fn logical_class_family_refuses_defaults_missing_and_stale_source_context() {
        // naming.diagnostics.retained-logical-class-family
        // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
        let mut analyser = inspected_analysis("tcl");
        assert!(family_is_tcloo(&analyser));
        let original = analyser.result.all_classes.get("::Dog").unwrap().clone();
        analyser
            .result
            .all_classes
            .get_mut("::Dog")
            .unwrap()
            .metaclass_provenance = MetaclassProvenance::StandIn;
        assert!(!family_is_tcloo(&analyser));
        analyser
            .result
            .all_classes
            .insert("::Dog".into(), original.clone());
        analyser
            .result
            .all_classes
            .get_mut("::Dog")
            .unwrap()
            .source_name_ambiguous = SourceNameAmbiguity::Observed;
        assert!(!family_is_tcloo(&analyser));
        analyser.result.all_classes.insert("::Dog".into(), original);
        analyser.source.push_str("# changed whole input");
        assert!(!family_is_tcloo(&analyser));
        analyser.source = SOURCE.to_owned();
        analyser.profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").unit_profile();
        analyser.result.dialect = "tcl9.1".into();
        assert!(
            family_is_tcloo(&analyser),
            "display profiles cannot replace the retained context"
        );
        let retained_context = analyser.context.clone();
        let unavailable =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        analyser.context = Some(Arc::new(
            unavailable.with_command_store(retained_context.as_ref().unwrap().commands().clone()),
        ));
        assert!(
            !family_is_tcloo(&analyser),
            "same store with another context is stale"
        );
        analyser.context = retained_context;
        let retained_input = analyser.result.resolved_input.take();
        assert!(
            !family_is_tcloo(&analyser),
            "a copied lexical flag supplies no Logical input"
        );
        analyser.result.resolved_input = retained_input;
        assert!(family_is_tcloo(&analyser));
    }

    #[test]
    fn native_and_hosted_family_classification_cannot_fall_back_to_logical_reports() {
        // naming.diagnostics.retained-logical-class-family
        // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
        for dialect in [
            "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl", "irules", "tmsh",
        ] {
            let mut analyser = inspected_analysis(dialect);
            assert!(
                !analyser.result.allows_retained_logical_declaration_advice(),
                "{dialect}"
            );
            assert!(
                analyser
                    .retained_logical_class_definer_grammar("::Dog")
                    .is_none(),
                "{dialect}"
            );
            if let Some(class) = analyser.result.all_classes.get_mut("::Dog") {
                // Retain every reporting field but remove its canonical source producer.
                class.source_name = None;
                assert!(
                    !family_is_tcloo(&analyser),
                    "{dialect}: reports cannot donate a Native class receipt"
                );
            }
        }
    }
}

#[cfg(test)]
mod original_expression_assistance_tests {
    use super::*;

    fn input(braced_var: tcl_dialect::BracedVarStyle) -> crate::analyser::ResolvedAnalysisInput {
        let selected =
            tcl_dialect::DialectProfile::find("tcl8.4").expect("authored Tcl 8.4 grammar");
        let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
        profile.grammar = selected.grammar;
        profile.expr_grammar_base = selected.expr_grammar_base;
        let profile = profile.intern();
        let config = tcl_lexer::LexerConfig {
            braced_var,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            config,
        );
        assert!(input.has_logical_source_name_context());
        input
    }

    #[test]
    fn original_operator_assistance_preserves_retained_variable_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Recognition and warning geometry only; no Native syntax acceptance.
        let source = "expr {${a{b}c} in {λ}}";
        for (style, count) in [
            (tcl_dialect::BracedVarStyle::FirstClose, 0),
            (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
        ] {
            let result = Analyser::new()
                .with_resolved_input(input(style))
                .analyse(source, "tcl");
            let warnings = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W003)
                .collect::<Vec<_>>();
            assert_eq!(warnings.len(), count, "{style:?}: {:?}", result.diagnostics);
            if let [warning] = warnings.as_slice() {
                assert_eq!(source.get(warning.span.as_range()), Some("in"));
            }
        }
    }

    #[test]
    fn original_function_assistance_keeps_lexer_overlays_and_missing_owner_refusals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original source references only, independently of dispatch/frames.
        let source = "expr {max(${a{b}c}, 1)}";
        for (style, count) in [
            (tcl_dialect::BracedVarStyle::FirstClose, 0),
            (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
        ] {
            let actual = input(style);
            let mut analyser = Analyser::new().with_resolved_input(actual.clone());
            let result = analyser.analyse(source, "tcl");
            let functions = result
                .command_invocations
                .iter()
                .filter(|invocation| invocation.is_mathfunc_call && invocation.name == "max")
                .collect::<Vec<_>>();
            assert_eq!(functions.len(), count, "{style:?}");
            if let [function] = functions.as_slice() {
                assert_eq!(source.get(function.range.as_range()), Some("max"));
                assert_eq!(function.argc, Some(2));
            }
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                source,
                0,
                actual.lexer_config(),
            )
            .pop()
            .unwrap();
            let expression = command.argv[1];
            analyser.result.resolved_input = None;
            assert!(analyser.expr_function_calls(expression).is_empty());
            analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                actual.analyser_profile(),
                actual.unit_profile(),
                tcl_registry::model::ingress::resolve_environment("tcl9.0")
                    .default_context_registry(),
                actual.lexer_config(),
            ));
            assert!(analyser.expr_function_calls(expression).is_empty());
            let mut stale = actual;
            stale.config.strict_quoting = !stale.config.strict_quoting;
            analyser.result.resolved_input = Some(stale);
            assert!(analyser.expr_function_calls(expression).is_empty());
        }
    }
}

#[cfg(test)]
mod configured_proposal_controls {
    use crate::analyser::{Analyser, ResolvedAnalysisInput};
    use tcl_core_types::DiagCode;

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            config,
        )
    }

    #[test]
    fn actual_formal_and_lambda_repairs_are_complete_configured_words() {
        for source in ["proc target {{a b c}} {}", "apply {{{a b c}} {return 1}}"] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            config.strict_quoting = !config.strict_quoting;
            let actual = ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment("tcl8.6")
                    .default_context_registry(),
                config,
            );
            let result = Analyser::new()
                .with_resolved_input(actual.clone())
                .analyse(source, "tcl");
            let diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == DiagCode::E006)
                .unwrap();
            let [fix] = diagnostic.fixes.as_slice() else {
                panic!("original repair: {source}")
            };
            let expected = if source.starts_with("proc") {
                "a b c".to_owned()
            } else {
                tcl_syntax::list::join_list(["a b c", "return 1"])
            };
            assert!(tcl_syntax::word_rules::lexical_unicode_source_word_matches(
                &fix.new_text,
                &expected,
                actual.lexer_config(),
            ));
            assert_eq!(fix.span, diagnostic.span);
        }
    }

    #[test]
    fn selector_and_option_proposals_keep_written_origin_separate_from_capture() {
        for (written, captured, code) in [
            (
                "string indez abc 0",
                "interp alias {} selected {} string indez; selected abc 0",
                DiagCode::W001,
            ),
            (
                "lsort -d {b a}",
                "interp alias {} selected {} lsort -d; selected {b a}",
                DiagCode::W145,
            ),
        ] {
            let actual = input();
            let direct = Analyser::new()
                .with_resolved_input(actual.clone())
                .analyse(written, "tcl");
            let diagnostic = direct
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .unwrap();
            assert!(!diagnostic.fixes.is_empty(), "{written}");
            for fix in &diagnostic.fixes {
                let plan = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::document(&fix.new_text),
                    tcl_lexer::Span::new(0, u32::try_from(fix.new_text.len()).unwrap()),
                    actual.lexer_config(),
                )
                .unwrap();
                assert_eq!(plan.commands.len(), 1);
                assert_eq!(plan.commands[0].words.len(), 1);
                assert!(plan.fatal_tail.is_none());
            }
            let indirect = Analyser::new()
                .with_resolved_input(actual)
                .analyse(captured, "tcl");
            let diagnostic = indirect
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .unwrap();
            assert!(diagnostic.fixes.is_empty(), "{captured}");
        }
    }
}

#[cfg(test)]
mod original_private_tcl_policy_controls {
    use super::*;

    #[test]
    fn actual_source_policy_keeps_unknown_helpers_and_whole_written_fix_boundary() {
        for dialect in ["tcl8.6", "tcl"] {
            for (source, fixed) in [
                ("::tcl::dict::create a 1", true),
                ("{::tcl::dict::create} a 1", false),
                ("::tcl::clock::GetSystemTimeZone", false),
            ] {
                let result = Analyser::new().analyse(source, dialect);
                let diagnostic = result
                    .diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.code == DiagCode::W143)
                    .unwrap();
                let subject = diagnostic.private_tcl_namespace().unwrap();
                assert!(subject.matches_source(source, &result));
                assert!(!subject.matches_source(&format!("{source} changed"), &result));
                assert_eq!(!diagnostic.fixes.is_empty(), fixed);
                assert!(subject.obligations().contains(
                    &crate::analyser::SourcePrivateTclNamespaceObligation::UnavailableActualLookup
                ));
                if source.contains("GetSystemTimeZone") {
                    assert!(subject.selected_source_words().is_none());
                }
            }
        }
    }

    #[test]
    fn actual_original_declaration_and_require_own_suppression_not_report_names() {
        for dialect in ["tcl8.6", "tcl"] {
            for source in [
                "proc ::tcl::dict::mine {d} {return $d}; ::tcl::dict::mine value",
                "::tcl::dict::mine value; package require tcl::dict",
                "package require tcl::dict; ::tcl::dict::mine value",
            ] {
                let result = Analyser::new().analyse(source, dialect);
                assert!(
                    !result
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagCode::W143),
                    "{source}: {dialect}"
                );
            }
            let result = Analyser::new().analyse(
                "package require :tcl::dict; ::tcl::dict::mine value",
                dialect,
            );
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagCode::W143)
            );
        }
    }
}
