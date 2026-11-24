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

//! The diagnostic emitters run at each dispatch site: procedure resolution, injection, registry arguments, bounds, numerals and expression operands.

use super::super::state::Analyser;
use super::super::types::{CodeFix, Severity};
use super::DispatchSite;
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::ArgRole;

impl Analyser {
    /// Run E006 for the argument shapes the active command spec identifies as
    /// formal lists and static-variable lists. This is deliberately a
    /// original Registry/declaration query, including resolver-defined roles,
    /// effective argv lineage and nested lambda literals. Known replacements
    /// cannot borrow a nominal builtin formal-list role.
    fn emit_formal_parameter_list_diagnostics(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticSource>,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        let Some(original) = original else {
            return;
        };
        // Effective source roles belong to this retained Registry/declaration
        // issuer. Captures have no direct-written ordinal in this argument slice.
        let roles = original
            .argument_roles()
            .into_iter()
            .filter_map(|(ordinal, role)| Some((original.written_index(ordinal)?, role)))
            .collect::<Vec<_>>();
        let indices = |wanted| {
            roles
                .iter()
                .filter_map(|(index, role)| (*role == wanted).then_some(*index))
                .collect::<Vec<_>>()
        };
        let parameter_indices = indices(ArgRole::ParamList);
        super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            self,
            args,
            arg_tokens,
            &parameter_indices,
        );
        let static_indices = indices(ArgRole::StaticVarList);
        super::super::diagnostics::emit_invalid_static_variable_list_diagnostics(
            self,
            args,
            arg_tokens,
            &static_indices,
        );
        let lambda_indices = indices(ArgRole::LambdaLiteral);
        super::super::diagnostics::emit_invalid_lambda_parameter_list_diagnostics(
            self,
            args,
            arg_tokens,
            &lambda_indices,
        );
    }

    /// Emit standalone-keyword and hosted source-procedure call diagnostics:
    ///
    /// - **W125** (Warning) — an orphaned control-flow keyword
    ///   (`else` / `elseif` / `then` / `on` / `trap` / `finally`) used as a
    ///   standalone command.  This almost always means a misplaced newline
    ///   split its parent `if` / `try` (`}\nelse {` instead of `} else {`).
    ///   Requires positive diagnostic absence at the original lookup point;
    ///   occupied or unknown source slots suppress the warning.
    /// - **IRULE5005** (Error) — a user proc invoked directly inside an
    ///   iRules event body, where Tcl-on-TMM requires `call PROC ARGS`.
    ///   Ships a `call PROC`-rewrite [`CodeFix`].
    pub(in super::super) fn emit_proc_resolution_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if let Some(parent) = tcl_registry::clause_grammar::owner_of_keyword(cmd_name)
            && self.original_keyword_slot_is_absent(cmd_name, cmd_tok)
        {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W125,
                    cmd_tok.span,
                    format!(
                        "\"{cmd_name}\" used as standalone command — should be part of \
                     \"{parent}\" (check for misplaced newline)"
                    ),
                    Severity::Warning,
                ));
        }

        let irules_proc_dispatch_context = self.profile.is_irules()
            && matches!(
                self.irules_execution_context(scope_path),
                tcl_registry::events::IrulesExecutionContext::EventBody
                    | tcl_registry::events::IrulesExecutionContext::ProcedureBody
            );
        if irules_proc_dispatch_context && self.resolve_proc_call(cmd_name, scope_path).is_some() {
            let suffix = if args.is_empty() {
                String::new()
            } else {
                format!(" {}", args.join(" "))
            };
            self.result.diagnostics.push(
                crate::analyser::types::Diagnostic::new(
                    DiagCode::Irule5005,
                    cmd_tok.span,
                    format!("iRules procs must be invoked with 'call': call {cmd_name}{suffix}"),
                    Severity::Error,
                )
                .with_fixes(vec![CodeFix {
                    span: cmd_tok.span,
                    new_text: format!("call {cmd_name}"),
                    description: format!("Use 'call {cmd_name}'"),
                    // IRULE5005: routing the call through `call` is required for an
                    // iRules proc, but the analyser cannot prove the head resolves to a
                    // proc rather than to a renamed or aliased command.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                }]),
            );
        }
    }

    /// Missing reporting metadata cannot establish an orphaned keyword.
    /// The original lookup owner retains occupants and all unknown alternatives.
    pub(super) fn original_keyword_slot_is_absent(&self, command: &str, token: Token) -> bool {
        let Some(input) = self.result.resolved_input.as_ref() else {
            return false;
        };
        let image = self.command_document_source_image();
        if !self
            .result
            .matches_original_source_image(&image, input.lexer_config())
            || !self.head_identities.matches_resolved_analysis_input(input)
        {
            return false;
        }
        let Some((word, offset)) = self
            .head_identities
            .original_written_word_at_span_in_source(&image, token.span, input.lexer_config())
        else {
            return false;
        };
        if offset != token.span.start()
            || word.span().start() != offset
            || word.group().expand
            || tcl_syntax::word_rules::original_static_word_unicode_value(&word).as_deref()
                != Some(command.as_bytes())
        {
            return false;
        }
        self.head_identities
            .diagnostic_slot_presence_at(offset)
            .knowledge
            == tcl_core_types::Knowledge::Absent
    }

    /// The script-injection family — a script or command line built by
    /// concatenating substituted words, which the interpreter then re-parses.
    ///
    /// Grouped out of [`Self::emit_dispatch_site_diagnostics`] because they
    /// are one subject with one shared argument shape, and because that
    /// dispatcher is at its line budget.
    fn emit_injection_diagnostics(
        &mut self,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        self.emit_w101_eval_string_concat(original);
        self.emit_w102_subst_injection(original);
        self.emit_w103_open_pipeline(original);
        self.emit_w300_source_variable(original);
        self.emit_w309_eval_subst_double_decode(original);
        self.emit_w301_uplevel_injection(original);
        self.emit_w105_original_script_words(original);
        self.emit_w312_interp_eval_injection(original);
        self.emit_w303_redos(original);
    }

    /// Registry-owned literal/value diagnostics. Kept as one dispatch-site
    /// group so adding a registry validator does not grow the main diagnostic
    /// dispatcher; none of these consumers knows a command name.
    fn emit_registry_argument_diagnostics(
        &mut self,
        _site: &DispatchSite<'_>,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        self.emit_w127_closed_value_args(original);
        self.emit_w127_closed_option_values(original);
        self.emit_w146_literal_argument_validation(original);
    }

    /// The bounds family for one dispatch site: the loop-termination
    /// candidate (W240 / W241 / W242, resolved once the CFG/SSA pass has the
    /// solver's branch facts), W230 / W232 index bounds, W231 `lset` bounds,
    /// and W232 string indices.
    ///
    /// Loop plans use the actual metadata context. W230/W232 require the
    /// selected original source schema and preserve captured operand origins;
    /// the separate W231 prior-value scan receives the same retained context.
    fn emit_bounds_family_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let generation = self.analysis_context();
        let registry = super::super::bounds_checks::BoundsMetadataContext::Retained(&generation);
        let grammar = self.grammar();
        let surface = self.command_surface(generation.commands());
        let candidate = if surface.declares(cmd_name) {
            super::super::bounds_checks::loop_termination_candidate(
                cmd_name,
                args,
                arg_tokens,
                registry,
                Some(&surface),
                self.file_lexer_config(),
                &grammar,
            )
        } else {
            original.and_then(|original| {
                super::super::bounds_checks::original_loop_termination_candidate(
                    original,
                    &self.result,
                    &self.source,
                )
            })
        };
        if let Some(mut candidate) = candidate {
            cmd_name.clone_into(&mut candidate.cmd_name);
            self.loop_candidates.push(candidate);
        }
        let numbers = grammar.numbers;
        let idx_diags = original.map_or_else(
            Vec::new,
            super::super::bounds_checks::original_index_diagnostics,
        );
        let lset_diags = super::super::bounds_checks::lset_index_diagnostics(
            cmd_name,
            args,
            arg_tokens,
            &self.source,
            registry,
            self.file_lexer_config(),
            // The numerals the document is lexed under, which an ingress
            // grammar may set apart from the profile's.
            crate::interval_bounds::BoundsRelease {
                numbers,
                ..self.bounds_release()
            },
        );
        self.result.diagnostics.extend(idx_diags);
        self.result.diagnostics.extend(lset_diags);
    }

    /// Dispatch-site diagnostic emitters, run from
    /// [`Self::process_command`] before the early-returning handlers so
    /// option-bearing / body-owning commands still get checked.
    ///
    /// - **W302** (`catch` without a result variable) — fires before
    ///   the early-returning `handle_catch_command`.
    /// - **W001** (unknown subcommand on a `SubcommandSig` command) —
    ///   before `handle_namespace_eval_command` so `namespace foo` is
    ///   flagged.
    /// - **E004** (malformed `if`) — dispatched generically off the
    ///   registry's structural defect for the call (the clause grammar's
    ///   walk for a command whose arity is checked structurally, else the
    ///   `clause_shape_check` escape hatch), not off `cmd_name`, so `if` is
    ///   the trigger today only because it is the one command whose arity
    ///   its grammar owns.
    /// - **W101** (`eval` with substituted args) — before body-walk
    ///   dispatch so the `ArgRole::Body` recursion into the `eval`
    ///   body still runs.
    /// - **W304** (missing `--` option terminator) — driven by the
    ///   registry's option-terminator profile.
    /// - **W004** (option not available in the active dialect).
    /// - **E002 / E003** (arity) — collected here and flushed
    ///   post-walk by [`Self::flush_arity_diagnostics`].
    /// - **W143** (direct call into a private `::tcl::` implementation
    ///   namespace) — dialect-independent, registry-driven.
    pub(super) fn emit_dispatch_site_diagnostics(&mut self, site: &DispatchSite<'_>) {
        let original = self.original_diagnostic_invocation(site);
        let format_templates = Self::original_format_templates(original.as_ref());
        let DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            arg_expand_in,
            cmd_tok,
            scope_path,
            presubstituted_args,
        } = *site;
        let formal_source = self.original_diagnostic_source(site);
        self.emit_formal_parameter_list_diagnostics(formal_source.as_ref(), args, arg_tokens);
        self.emit_w302_catch_no_result_var(original.as_ref());
        self.emit_w001_unknown_subcommand(original.as_ref());
        if original.is_none() {
            self.record_widget_dispatch_candidate(
                cmd_name,
                args,
                cmd_tok,
                arg_tokens,
                arg_expand_in,
            );
        }
        let unavailable = if presubstituted_args || original.is_some() {
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_COMMAND_AVAILABILITY").is_some() {
                eprintln!(
                    "COMMAND_AVAILABILITY offset={} stage=dispatch.domain passed=false presubstituted={presubstituted_args} selected={}",
                    cmd_tok.span.start(),
                    original.is_some()
                );
            }
            None
        } else {
            let tokens = self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens);
            if tokens.is_none()
                && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_COMMAND_AVAILABILITY").is_some()
            {
                eprintln!(
                    "COMMAND_AVAILABILITY offset={} stage=dispatch.original-tokens passed=false",
                    cmd_tok.span.start()
                );
            }
            tokens.and_then(|tokens| {
                crate::registry_invocation::original_source_command_availability(
                    &self.source,
                    &self.result,
                    &tokens,
                )
            })
        };
        self.emit_w002_disabled_command(unavailable, scope_path);
        self.emit_e004_clause_shape_diagnostic(original.as_ref());
        self.emit_w142_context_gate(original.as_ref());
        self.emit_injection_diagnostics(original.as_ref());
        self.emit_w306_literal_expected(original.as_ref());
        // Default credential flags are lexical policy. Registry extensions
        // require the same original selected descriptor and effective operands.
        self.emit_w310_hardcoded_credentials(
            original.as_ref(),
            args,
            arg_tokens,
            (!presubstituted_args).then_some(cmd_tok.span.start()),
        );
        // W143: direct call into a private `::tcl::` implementation
        // namespace.  Deferred — the whole-file suppressions
        // are applied by `flush_w143_diagnostics`.
        self.emit_w143_private_tcl_namespace(cmd_name, cmd_tok, scope_path);
        // IRULE2002: deprecated iRules command (f5-irules only).
        self.emit_irule2002_deprecated_command(original.as_ref());
        // IRULE2001: deprecated `matchclass` (f5-irules only).  Fires
        // alongside IRULE2002 at the same command-head span.
        self.emit_source_deprecation_advice(original.as_ref());
        // IRULE1003 / 1004 / 2101 / 4001 / 4003 / 5001 / 6001 —
        // analyser-level iRules event-context checks (f5-irules only).
        self.emit_irules_event_checks(cmd_name, args, arg_tokens, arg_single, cmd_tok, scope_path);
        // TK1001 / TK1002 / TK1003 — Tk-dialect widget + geometry checks
        // (tk dialect only); the TK1001 conflict is flushed post-walk.
        self.emit_tk_checks(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_source_variable_name_advice(site);
        self.emit_w104_append_list(original.as_ref());
        self.emit_w106_unbraced_switch_body(original.as_ref());
        self.emit_w311_encoding_mismatch(original.as_ref());
        self.emit_binary_field_version_gates(&format_templates);
        self.emit_w121_invalid_subnet_mask(args, arg_tokens, original.as_ref());
        self.emit_w108_non_ascii(arg_tokens);
        self.emit_w148_numeral_release(args, arg_tokens);
        self.emit_w151_range_numerals(args, arg_tokens);
        self.emit_bounds_family_diagnostics(cmd_name, args, arg_tokens, original.as_ref());
        self.emit_registry_argument_diagnostics(site, original.as_ref());
        self.emit_w304_missing_option_terminator(original.as_ref(), cmd_name);
        self.emit_w217_unset_option_only(original.as_ref());
        self.emit_w004_dialect_invalid_option(original.as_ref(), cmd_name);
        // W135 / W136 — command/option needs a newer package version than the
        // resolved `package require` floor (buffered, decided post-walk).
        self.record_version_gate_sites(original.as_ref(), cmd_name);
        // W138 — format/scan %-string conversions gated behind a Tcl
        // release (buffered, decided post-walk — §6 argument-DSL rung).
        self.record_dsl_format_sites(cmd_name, &format_templates);
        self.emit_source_signature_advice(site, original.as_ref());
        if !presubstituted_args && original.is_some() {
            self.record_proven_site(&super::super::diagnostics::CallWords {
                cmd_tok,
                args,
                arg_tokens,
                arg_single,
                arg_expand_in,
            });
        }
    }

    /// Each genuine source signature retains its independent descriptor.
    fn emit_source_signature_advice(
        &mut self,
        site: &DispatchSite<'_>,
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        self.emit_arity_diagnostics(site.cmd_name, site.cmd_tok, site.scope_path, original);
        if original.is_none()
            && let Some(declared) = self.original_declared_diagnostic_invocation(site)
        {
            self.emit_declared_source_arity(declared, site.cmd_name);
        }
    }

    /// One genuine complete original invocation vector for independently
    /// selected Registry syntax and document-declared source contracts.
    fn emit_source_variable_name_advice(&mut self, site: &DispatchSite<'_>) {
        // W212 asks whether a *written* variable-name word was spelled as a
        // `$` substitution by mistake.  In a `list`-built script that
        // question does not arise: `uplevel 1 [list set $var 99]` substitutes
        // `$var` in the building frame, so the script `uplevel` finally runs
        // already carries the literal name the user meant, and the
        // substitution is the whole point of the idiom (tclsh 9.0.4 / 8.6.16:
        // `proc setInCaller {var} {uplevel 1 [list set $var 99]}` /
        // `proc useIt {} {setInCaller answer; return $answer}` prints `99`).
        // Only the directly-written spelling (`set $var 99`) is the
        // name/value confusion the code is about.
        if site.presubstituted_args {
            return;
        }
        let source = self.original_diagnostic_source(site);
        self.emit_w212_name_vs_value(source.as_ref(), site.cmd_name, site.scope_path);
    }

    /// Report modern numeral spellings which the resolved document grammar
    /// rejects (W148). Dynamic/substituted words and words that are not valid
    /// Tcl 9 numerals are deliberately ignored.
    fn emit_w148_numeral_release(&mut self, args: &[String], tokens: &[Token]) {
        let syntax = self.grammar().numbers;
        for (arg, token) in args.iter().zip(tokens.iter()) {
            if !matches!(token.kind, TokenType::Str | TokenType::Esc)
                || arg.contains('$')
                || arg.contains('[')
                || tcl_syntax::number::parse_whole_with(
                    arg.trim(),
                    tcl_syntax::number::ParseFlags::for_syntax(tcl_dialect::NumberSyntax::Tcl90),
                )
                .is_none()
                || tcl_syntax::number::parse_whole_with(
                    arg.trim(),
                    tcl_syntax::number::ParseFlags::for_syntax(syntax),
                )
                .is_some()
            {
                continue;
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W148,
                    token.span,
                    format!(
                        "Numeral '{}' is not accepted by the resolved Tcl numeral grammar.",
                        arg.trim()
                    ),
                    Severity::Warning,
                ));
        }
    }

    /// §5.4 cross-target numeral portability (**W151**): a literal word
    /// that does not read as the *same* number under every numeral
    /// grammar the declared target range spans.
    ///
    /// Two divergence shapes, both from the numeral axis the dialect
    /// model already carries ([`tcl_dialect::NumberSyntax`], §3.1):
    ///
    /// * **meaning** — the word parses everywhere but to different
    ///   values: the motivating leading-zero case (`010` is 8 under
    ///   Tcl 8.x targets and 10 under 9.0 — a silent behaviour change);
    /// * **validity** — the word is a numeral under some targets only
    ///   (`0b101`/`0o17` predate 8.5; `0d5` and `_` separators predate
    ///   9.0).
    ///
    /// A word the **primary** grammar itself rejects is W148's
    /// single-target fact and is skipped here — one word, one
    /// diagnostic. Dynamic/substituted words are ignored exactly as
    /// W148 ignores them, and the whole walk is off unless the declared
    /// range spans at least two numeral grammars
    /// ([`Analyser::range_numeral_grammars`]).
    fn emit_w151_range_numerals(&mut self, args: &[String], tokens: &[Token]) {
        if self.range_numeral_grammars.len() < 2 {
            return;
        }
        let grammars = self.range_numeral_grammars.clone();
        let primary = self.grammar().numbers;
        let mut new_diags: Vec<crate::analyser::types::Diagnostic> = Vec::new();
        for (arg, token) in args.iter().zip(tokens.iter()) {
            if !matches!(token.kind, TokenType::Str | TokenType::Esc)
                || arg.contains('$')
                || arg.contains('[')
            {
                continue;
            }
            let word = arg.trim();
            if word.is_empty() {
                continue;
            }
            let read = |syntax: tcl_dialect::NumberSyntax| {
                tcl_syntax::number::parse_whole_with(
                    word,
                    tcl_syntax::number::ParseFlags::for_syntax(syntax),
                )
            };
            let readings: Vec<(
                tcl_dialect::NumberSyntax,
                Option<tcl_syntax::number::Number>,
            )> = grammars
                .iter()
                .map(|&syntax| (syntax, read(syntax)))
                .collect();
            if readings.iter().all(|(_, reading)| reading.is_none()) {
                continue;
            }
            // The primary's own rejection is W148's fact, not a range one.
            if read(primary).is_none() {
                continue;
            }
            let message = if let Some((rejecting, _)) =
                readings.iter().find(|(_, reading)| reading.is_none())
            {
                format!(
                    "Numeral '{word}' is not accepted under the declared target(s) {}.",
                    self.range_numeral_target_names(*rejecting)
                )
            } else {
                let (first_grammar, first) = &readings[0];
                let Some((diverging, other)) = readings
                    .iter()
                    .find(|(_, reading)| reading != first)
                    .map(|(grammar, reading)| (*grammar, reading.clone()))
                else {
                    continue; // every target reads the same value
                };
                let spell = |reading: &Option<tcl_syntax::number::Number>| match reading {
                    Some(tcl_syntax::number::Number::Int(value)) => value.to_string(),
                    _ => "a different value".to_owned(),
                };
                format!(
                    "Numeral '{word}' means {} under declared target(s) {} but {} under {} — a silent meaning change across the declared range.",
                    spell(first),
                    self.range_numeral_target_names(*first_grammar),
                    spell(&other),
                    self.range_numeral_target_names(diverging)
                )
            };
            new_diags.push(crate::analyser::types::Diagnostic::new(
                DiagCode::W151,
                token.span,
                message,
                Severity::Warning,
            ));
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// The declared core-target names a W151 message cites for the
    /// targets that resolve `grammar` — the declared set intersected
    /// with the grammar's era, named at ladder granularity.
    fn range_numeral_target_names(&self, grammar: tcl_dialect::NumberSyntax) -> String {
        use tcl_dialect::model::{Family, VersionAxisId};
        let axis = VersionAxisId::core(Family::Tcl);
        // JimTcl is a reimplementation, not a point on the Tcl ladder, so
        // its numeral grammars have no interval on the core axis to
        // intersect a declared range with. Name the Jim releases directly.
        match grammar {
            tcl_dialect::NumberSyntax::Jim => return "JimTcl 0.76-0.79".to_owned(),
            tcl_dialect::NumberSyntax::Jim080 => return "JimTcl 0.80+".to_owned(),
            tcl_dialect::NumberSyntax::Tcl84
            | tcl_dialect::NumberSyntax::Tcl85
            | tcl_dialect::NumberSyntax::Tcl90 => {}
        }
        let fallback = || {
            format!(
                "Tcl {}",
                match grammar {
                    tcl_dialect::NumberSyntax::Tcl84 => "8.4",
                    tcl_dialect::NumberSyntax::Tcl85 => "8.5",
                    // Unreachable: the Jim arms returned above.
                    _ => "9.0",
                }
            )
        };
        let Some(declared) = self
            .range_context
            .as_ref()
            .and_then(|context| context.declared_targets(&axis))
        else {
            return fallback();
        };
        let requirement = match grammar {
            tcl_dialect::NumberSyntax::Tcl84 => "0-8.5",
            tcl_dialect::NumberSyntax::Tcl85 => "8.5-9.0",
            // Unreachable: the Jim arms returned above.
            _ => "9.0-",
        };
        let Ok(era) =
            tcl_dialect::model::VersionSet::from_requirements(axis.clone(), &[requirement])
        else {
            return fallback();
        };
        let Ok(overlap) = declared.intersect(&era) else {
            return fallback();
        };
        let names = tcl_registry::model::ladder_releases_in(&overlap);
        if names.is_empty() {
            fallback()
        } else {
            names.join(", ")
        }
    }

    /// Diagnose expression operands from the retained original source schema.
    /// Registry roles, selected whole-tail grammar and written ordinals share
    /// one owner; captured and expanded operands cannot become source anchors.
    pub(super) fn dispatch_expr_arguments(&mut self, site: &DispatchSite<'_>) {
        let Some(original) = self.original_diagnostic_source(site) else {
            return;
        };
        let Some(expressions) = original.expression_arguments() else {
            return;
        };
        if expressions.arguments.is_empty() {
            return;
        }
        self.emit_w100_unbraced_expr(&original, &expressions);
        if expressions.concatenates && original.arguments().len() > 1 {
            if let Some(registry) = original.registry() {
                self.dispatch_joined_source_expression(site, registry);
            }
            return;
        }
        for argument in expressions.arguments {
            let Some(index) = original.written_index(argument) else {
                continue;
            };
            let (Some(word), Some(text), Some(token)) = (
                original.word(argument),
                site.args.get(index),
                site.arg_tokens.get(index),
            ) else {
                continue;
            };
            self.emit_w110_string_eq_ne(
                text,
                word.span(),
                &super::super::diagnostics::W110Anchor::ArgToken(*token),
            );
            if let Ok(content) = word.content_span() {
                self.emit_w003_dialect_invalid_expr_operator(content);
            }
            self.emit_expr_function_dialect_diagnostics(*token);
            self.emit_w114_original_nested_expr(&original, argument);
            self.emit_w154_precedence_hazards(word);
        }
    }

    /// Whole-tail expression assistance needs an entirely written original
    /// vector. Literal source values may be joined; dynamic words do not prove
    /// the runtime expression received by Tcl and are handled by W100 instead.
    fn dispatch_joined_source_expression(
        &mut self,
        site: &DispatchSite<'_>,
        original: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
    ) {
        let count = original.words().arguments().len();
        let Some(last) = count.checked_sub(1) else {
            return;
        };
        if (0..count).any(|index| original.written_index(index) != Some(index)) {
            return;
        }
        let Some(first_word) = original.word(0) else {
            return;
        };
        let Some(last_word) = original.word(last) else {
            return;
        };
        let span = Span::new(first_word.span().start(), last_word.span().end());
        let values = (0..count)
            .map(|index| original.literal(index))
            .collect::<Option<Vec<_>>>();
        if let Some(values) = values {
            let expression = values.join(" ");
            self.emit_joined_precedence_hazards(&expression, first_word.config(), span);
            self.emit_w110_string_eq_ne(
                &expression,
                span,
                &super::super::diagnostics::W110Anchor::JoinedWords {
                    args: site.args,
                    tokens: site.arg_tokens,
                },
            );
        }
        // Standalone original operator words have authentic written anchors
        // even when another expression operand is dynamically substituted.
        self.emit_w003_dialect_invalid_expr_words(site.args, site.arg_tokens, &site.args.join(" "));
    }
}
