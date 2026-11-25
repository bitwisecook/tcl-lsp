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

//! Wrong argument counts: the verdict for one signature, the candidates queued during the walk and the post-walk flushes for commands, constructors and `next`.

use super::UserResolutionFacts;
use crate::analyser::state::Analyser;
use crate::analyser::types::{PendingUserCallArity, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::Arity;

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
pub(in super::super) fn source_excess_arguments(
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

pub(in super::super) fn source_arity_verdict(
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
/// arity check ([`super::super::var_command`]), so all three diagnostics carry
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
pub(in super::super) fn arity_verdict(
    display_name: &str,
    arity: Arity,
    nargs_min: usize,
    positional_any_expand: bool,
    span: tcl_lexer::Span,
    excess: Option<ExcessArgs>,
    usage: Option<&str>,
) -> Option<crate::analyser::types::Diagnostic> {
    let usage_suffix = usage.map_or_default(|u| format!(" — usage: {u}"));
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
pub(in super::super) fn shift_arity(arity: Arity, prepended: u16) -> Arity {
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

impl Analyser {
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
                self.pending_gated_bare_ensemble.push(
                    super::super::version_gate::GatedBareEnsemble {
                        axis,
                        windows: shape.windows,
                        fallback: shape.arity,
                        display_name: display_name.to_owned(),
                        original: original.clone(),
                    },
                );
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
        // An option the release lacks (`encoding convertfrom -profile` on
        // 8.6) is counted as a positional word; W004 already reports the
        // call, so its count is not a second finding.
        if original
            .with_schema(crate::analyser::diagnostic_registry::source_diagnostic_options)
            .flatten()
            .is_some_and(|scan| scan.options.iter().any(|selected| !selected.available))
        {
            return;
        }
        if !shape.windows.is_empty() {
            self.pending_gated_arity
                .push(super::super::version_gate::GatedArityCall {
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

    pub(in super::super) fn queue_source_arity_diagnostic(
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
    pub(in super::super) fn settle_builtin_verdicts(
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
                let advice = std::sync::Arc::new(
                    super::super::types::OriginalConstructorArityAdvice::assess(
                        candidate.original,
                        &self.result,
                        &self.source,
                    )?,
                );
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
        let Some(original) = super::super::types::OriginalConstructorArityCall::from_source(
            &self.source,
            &self.result,
            cmd_tok.span.start(),
            !self.scope_path_in_proc_body(scope_path),
        ) else {
            return;
        };
        self.pending_ctor_arity
            .push(super::super::types::PendingCtorArity {
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
        let Some(original) = super::super::types::OriginalLexicalNextCall::from_source(
            context,
            &self.result,
            &self.source,
            &cmd_tok,
        ) else {
            return;
        };
        self.pending_next_arity
            .push(super::super::types::PendingNextArity {
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
}
