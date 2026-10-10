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

//! Central command dispatch.
//!
//! Walks one segmented Tcl command and routes it through the
//! per-command handlers. Which handler family owns a command form is
//! registry data — the [`tcl_registry::hooks::AnalyserHookId`] stamped
//! on its `CommandSpec` / `SubCommand` — so the dispatch is a single
//! typed `match` ([`Analyser::dispatch_analyser_hook`]); adding a new
//! handler means adding a hook variant and stamping the spec.

use std::sync::Arc;

use tcl_core_types::DiagCode;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::ArgRole;

use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;

use super::state::Analyser;
use super::types::{CodeFix, Severity};

/// A command head collected from a `[...]` substitution / expression scan,
/// ready to push as a `command_invocations` entry:
/// `(name, head-span, argc, callback_arity, ensemble_subcommand_candidate)`.
/// `argc` is the nested call's statically-known argument count (`None` when
/// `{*}`-expanded); `callback_arity` is `Some` only for an
/// `ArgRole::CommandPrefix` callback head (so the callback arity check runs)
/// and `None` for an ordinary call head. `ensemble_subcommand_candidate` is
/// `Some((word, span))` when the head's first actual argument is a static,
/// non-`{*}`-expanded word — a *candidate* subcommand
/// [`Analyser::push_collected_heads`] checks against
/// `ensemble_subcommand_targets` once the head's own resolved name is known
/// (a `[widget make hello]` nested call needs the same
/// subcommand-reference recording a top-level `widget make hello` call
/// already gets from [`Analyser::record_ensemble_subcommand_invocation`]).
struct CollectedHead {
    name: String,
    span: Span,
    argc: Option<usize>,
    callback: Option<crate::command_binding::OriginalCallbackPrefix>,
    ensemble: Option<(String, Span)>,
}

/// Maximum nested-body recursion depth for [`Analyser::analyse_body`].
/// `analyse_body` ↔ `process_command` ↔ `dispatch_body_arguments`
/// recurse one frame group per braced-body nesting level; generated /
/// minified Tcl and machine-emitted iRules can nest deeply enough to
/// overflow the stack — an uncatchable SIGABRT that crashes the LSP
/// diagnostics worker and the `tcl diag` / `lint` / `validate` CLIs. At
/// the cap we stop descending into further nested bodies (the diagnostics
/// already collected stand); no real source nests anywhere near this.
///
/// The number itself is [`crate::depth_guard::MAX_SOURCE_NEST_DEPTH`],
/// derived there from a stack budget and a measured per-level cost rather
/// than picked to match a convention.
const MAX_BODY_DEPTH: tcl_core_types::RecursionLimit = crate::depth_guard::MAX_SOURCE_NEST_DEPTH;

/// The borrowed word-level view of one command, threaded into the
/// dispatch-site diagnostic emitter ([`Analyser::emit_dispatch_site_diagnostics`]).
/// Bundling these co-varying slices keeps the emitter under the argument
/// limit; it destructures them back into locals so the per-code emitter
/// calls read unchanged.
struct DispatchSite<'a> {
    cmd_name: &'a str,
    args: &'a [String],
    arg_tokens: &'a [Token],
    arg_single: &'a [bool],
    arg_expand_in: &'a [bool],
    cmd_tok: Token,
    scope_path: &'a [usize],
    /// This command's words are pre-substituted `list` elements — see
    /// [`super::state::AnalyserState::presubstituted_args`].  A `$word` here
    /// was replaced by its value while the *building* frame ran, before the
    /// script existed, so it is not the written-source shape any
    /// substitution-vs-literal check is about.
    presubstituted_args: bool,
}

/// A readonly mini-language template from an original whole written word.
/// Registry selection owns its effective ordinal; the operand owns its value
/// and source anchor. Captured prefixes cannot acquire a call-site span.
pub(in crate::analyser) struct OriginalFormatTemplate {
    pub(in crate::analyser) format: tcl_registry::FormatStringArg,
    pub(in crate::analyser) bytes: Vec<u8>,
    pub(in crate::analyser) span: Span,
}

/// Bundled arguments for [`Analyser::record_var_or_cmd_command_site`] — the
/// command-head token plus the co-varying slices `process_command` and its
/// nested-substitution twin already have in hand.  Keeps the recorder under
/// the argument-count limit; it destructures them back into locals so the
/// body reads unchanged.  Every field is a reference or `Token` (itself
/// `Copy`), so the bundle is `Copy` too — passed by value, no
/// `needless_pass_by_value` lint to work around.
#[derive(Clone, Copy)]
struct VarOrCmdSite<'a> {
    cmd_name: &'a str,
    cmd_tok: Token,
    head_expanded: bool,
    args: &'a [String],
    arg_tokens: &'a [Token],
    arg_expand: &'a [bool],
    scope_path: &'a [usize],
}

/// Bundled arguments for [`Analyser::record_bareword_dispatch_site`] — the
/// `Esc`-head slice of [`VarOrCmdSite`], destructured back into locals in
/// the body.  `Copy` for the same reason [`VarOrCmdSite`] is: every field is
/// a reference or a `Copy` `Token`, so it passes by value with no
/// `needless_pass_by_value` lint to work around.
#[derive(Clone, Copy)]
struct BarewordDispatch<'a> {
    cmd_name: &'a str,
    cmd_tok: Token,
    args: &'a [String],
    method_span: Option<tcl_lexer::Span>,
    in_method: bool,
    arg_expand: &'a [bool],
}

/// A dispatched call's per-word source facts, parallel to its post-head
/// words.
#[derive(Clone, Copy)]
struct WordFacts<'a> {
    /// Whether each word is a single token.
    single: &'a [bool],
}

/// Hook and traits from one retained authoring-context resolution.
pub(super) struct ResolvedAnalyserHook {
    pub(super) hook: tcl_registry::hooks::AnalyserHookId,
    pub(super) traits: tcl_registry::Traits,
}

/// Which depths a body word raises while it is walked: `conditional_depth`
/// (branch-selected — nothing inside dominates the code after the command)
/// and `control_flow_body_depth` (not straight-line — it may run zero times
/// or many).
///
/// A body the call's clause plan places answers by its clause's timing: a
/// selected body is both, a per-iteration body or a loop's `next` fixture is
/// control flow, a protected body is a guarded probe, and a body that runs
/// exactly once whenever the call does (`for`'s `start`, `dict update`'s
/// body, `finally`) is neither. Any other body keeps the command's traits'
/// reading: `BRANCH_SELECTED_BODY` and `CONTROL_FLOW`.
fn body_depths(
    plan: Option<&tcl_registry::ClausePlan>,
    body: usize,
    traits: tcl_registry::Traits,
) -> (bool, bool) {
    use tcl_registry::{ClauseTiming, LoopPhase};
    let timing = plan.and_then(|plan| {
        plan.clauses
            .iter()
            .find(|clause| clause.operand(tcl_registry::arg_role::ArgRole::Body) == Some(body))
            .map(|clause| clause.timing)
    });
    match timing {
        Some(ClauseTiming::Selected) => (true, true),
        Some(ClauseTiming::PerIteration | ClauseTiming::LoopFixture(LoopPhase::Next)) => {
            (false, true)
        }
        Some(ClauseTiming::Protected) => (true, false),
        Some(ClauseTiming::Always | ClauseTiming::LoopFixture(LoopPhase::Init)) => (false, false),
        None => (
            traits.contains(tcl_registry::Traits::BRANCH_SELECTED_BODY),
            traits.contains(tcl_registry::Traits::CONTROL_FLOW),
        ),
    }
}

fn body_argument_indices(
    view: &crate::registry_invocation::InvocationBodyAssistance,
) -> Vec<usize> {
    view.possible_roles
        .iter()
        .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
        .collect()
}

fn is_dynamic_eval_body(view: &crate::registry_invocation::InvocationBodyAssistance) -> bool {
    view.definite_traits
        .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
        && view.definite_invocation.as_ref().is_some_and(|invocation| {
            tcl_registry::irules_policy::irules_disabled_class(&invocation.facts.canonical_command)
                .is_none()
        })
}

impl Analyser {
    /// Re-segment a body script and dispatch each command at
    /// `scope_path`.
    ///
    /// Used by every body-walking handler (`handle_proc_command`,
    /// `handle_switch_command`, `handle_catch_command`, the generic
    /// `dispatch_body_arguments`, etc.).
    ///
    /// Body recursion does **not** use the segmenter's re-segmentation
    /// recovery — that splits a runaway top-level command and only
    /// fires at the top level. The per-command syntax *detectors* (E100 /
    /// E102 stray closers, E201 unterminated `[`, E202 unterminated `"`,
    /// E203 unterminated `{`) do run on every body. Dynamic bodies
    /// (`$body`, `[gen]`) are skipped because they can't be statically
    /// re-segmented.
    ///
    /// `body_depth` is bumped for the duration of the walk so
    /// top-level-only command checks can distinguish nested
    /// invocations.
    pub(super) fn analyse_body(&mut self, body_text: &str, body_tok: Token, scope_path: &[usize]) {
        if body_tok.kind != TokenType::Str {
            // A selected source list builder can retain conditional command
            // syntax and its original operand reads. Missing parent/builder/
            // target ancestry leaves the body opaque; no frame follows from
            // the builder's presentation or returned-list shape alone.
            self.analyse_list_quoted_body(body_tok, scope_path);
            return;
        }
        self.body_depth += 1;
        if MAX_BODY_DEPTH.exceeded(self.body_depth) {
            // Stop descending before the recursive body walk overflows the
            // stack — the diagnostics collected up to this nesting level
            // still stand. Report it as a diagnostic (once per walk, not
            // once per nested body past the cap) rather than truncating
            // silently: tclsh's own recursion limit raises a catchable
            // "too many nested evaluations (infinite loop?)" error at this
            // point rather than continuing quietly, and a process abort is
            // never the right failure mode either way.
            if !self.structure_only && !self.e207_emitted {
                self.e207_emitted = true;
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::E207,
                        body_tok.span,
                        format!(
                            "nesting depth exceeds the analysis limit ({} levels) — \
                         diagnostics for this body and anything nested inside it are not \
                         collected",
                            MAX_BODY_DEPTH.0
                        ),
                        Severity::Error,
                    ));
            }
            self.body_depth -= 1;
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // The rebase below only produces truthful absolute spans while
        // `body_text` maps 1:1 onto its source region; clamp it to the part
        // that actually does.
        let body_text = crate::segmenter::body_text_in_region(
            &self.source,
            base_offset as usize,
            body_tok.span.end() as usize,
            body_text,
        );
        // Absolute byte offset at which this body region ends. The E202 /
        // E203 detectors test "reaches end of region" against this, not the
        // whole document — the body's tokens are absolute spans into
        // `self.source`, but a runaway `"` / `{` only swallows to the body's
        // own end.
        let region_end = base_offset as usize + body_text.len();
        let body_commands = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            base_offset,
            self.lexer_config(),
        );
        let total = body_commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &body_commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // An unterminated `"` / `{` emits the precise E202 /
                // E203 (with a closing-delimiter fix) ahead of the generic
                // E200, matching the top-level `walk_commands_top_level`
                // branch. Stolen-close-brace detection ⇒ E103
                // (brace partials only); otherwise the generic E200 fires so
                // the user still sees a parse-error diagnostic.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(crate::segmenter::UnclosedDelimiter::Brace)
                );
                if !(self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end)
                    || brace_partial && self.detect_stolen_close_brace(cmd_ref))
                {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // Run the E100 (stray `]`) / E102 (stray `}`) token
            // checks on every analysed body, not just the top
            // level. Run on the *original* token stream before
            // ``recover_stray_close_bracket`` repairs the clone,
            // matching the top-level loop's ordering.  Token spans
            // are absolute into the full document, so the full
            // ``self.source`` is the right slice base.
            let generation = self.analysis_context();
            let stray = super::syntax_checks::stray_closer_diagnostics(
                cmd_ref,
                &self.source,
                Some(generation.commands()),
                || self.user_command_tail_names(),
            );
            self.result.diagnostics.extend(stray);
            // E201 (unterminated `[`) inside a body — `proc p {} { set y
            // [foo }`.  The CST auto-closes the bracket so the command isn't
            // flagged `is_partial`, but the source carries no real `]`, so
            // it would otherwise go unreported.  Run the same E201
            // detector as `emit_syntax_recovery_diagnostics` at the
            // top level (the top-level ghost-recovery doesn't reach
            // into body scripts).
            let e201 = super::syntax_checks::unterminated_bracket_diagnostics(
                cmd_ref,
                &self.source,
                &self.recovery_known_commands,
            );
            self.result.diagnostics.extend(e201);
            // E202 (unterminated `"`) / E203 (unterminated `{`) inside a
            // body — `proc p {} { set x "\n puts hi }`.  The brace word the
            // body sits in is balanced, so the body re-segments cleanly and
            // the run-away quote/brace is a non-partial command whose token
            // reaches the body's end.  Run the same E202/E203 detector
            // as `emit_syntax_recovery_diagnostics` at the top level,
            // which the top-level ghost-recovery doesn't reach into body
            // scripts.
            self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end);
            let mut cmd = cmd_ref.clone();
            // Repair stray ``]`` (missing ``[``) so downstream
            // handlers see the intended argv shape before dispatch.
            self.recover_stray_close_bracket(&mut cmd);
            // Splice orphaned switch case pairs when ``{`` was
            // forgotten.  The returned count is added to ``cmd_idx``
            // so we skip past the consumed orphans.
            let consumed = self.recover_missing_open_brace(&mut cmd, &body_commands, cmd_idx);
            // ``# noqa`` directives in the preceding-comment
            // attribute to this command's line range — same
            // shape as the top-level loop.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &cmd.single_token_word,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                scope_path,
            );
            self.emit_w216_brace_then_paren(&cmd);
            // Record every `$var` substitution in arg positions so
            // `VarDef.references` carries the read spans the LSP
            // providers consume.
            self.record_arg_var_reads(&cmd, scope_path);
            cmd_idx += 1 + consumed;
        }
        self.body_depth -= 1;
    }

    /// Analyse one registry-declared control-flow body while recording that
    /// facts inside it may execute zero or multiple times.
    pub(super) fn analyse_control_flow_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
    ) {
        self.control_flow_body_depth += 1;
        self.analyse_body(body_text, body_tok, scope_path);
        self.control_flow_body_depth -= 1;
    }

    /// Walk conditional source syntax from a genuine selected list builder and
    /// its original parent/body and target schemas. All projected operands keep
    /// their builder extents. Execution requires its separately retained body
    /// invocation; source applicability cannot issue a frame or runtime argv.
    fn analyse_list_quoted_body(&mut self, body_tok: Token, scope_path: &[usize]) -> bool {
        if body_tok.kind != TokenType::Cmd {
            return false;
        }
        let config = self.lexer_config();
        let sm = SourceMap::new(&self.source);
        let child = descend_token(&sm, body_tok, config);
        if !child.is_terminated() {
            return false;
        }
        let commands = segments_from_tree(child.tree(), &sm);
        let [producer] = commands.as_slice() else {
            return false;
        };
        let Some(input) = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())
        else {
            return false;
        };
        let Some(words) =
            crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(
                &self.source,
                input,
                &self.head_identities,
                producer,
            )
        else {
            return false;
        };
        let Some(cmd) = crate::script_arg::original_list_built_script_command(&words, producer)
        else {
            return false;
        };
        self.body_depth += 1;
        if MAX_BODY_DEPTH.exceeded(self.body_depth) {
            self.body_depth -= 1;
            return false;
        }
        self.presubstituted_args = true;
        let evaluated = self
            .head_identities
            .executed_script_for_word(body_tok.span)
            .and_then(|source| {
                let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
                    &source.text,
                    source.base(),
                    self.lexer_config(),
                )?;
                let [command] = commands.as_slice() else {
                    return None;
                };
                let proof = self
                    .head_identities
                    .source_bindings()
                    .invocation_at_origin(&source.origin, command.argv.first()?.span.start());
                proof
                    .invocation_realm()
                    .map(|_| (cmd.argv[0].span.start(), proof))
            });
        let previous = std::mem::replace(&mut self.evaluated_body_invocation, evaluated);
        self.process_command(
            &cmd.texts,
            &cmd.argv,
            &cmd.single_token_word,
            &[],
            scope_path,
        );
        self.evaluated_body_invocation = previous;
        self.record_arg_var_reads(&cmd, scope_path);
        self.body_depth -= 1;
        true
    }

    /// The command head this call actually dispatches to, folding a
    /// statically-determined dynamic head to the name it names.
    ///
    /// A head written with a substitution is not automatically a runtime
    /// unknown: `set ns ticklecharts; ${ns}::setdef …` dispatches to
    /// `::ticklecharts::setdef` on every execution, and the `TclOO` idiom
    /// `set ns [namespace qualifiers [self class]]; ${ns}::setdef …` is the
    /// same shape one level removed (without the fold, go-to-definition,
    /// find-references, and call-hierarchy silently drop every such call
    /// site).  [`Analyser::resolve_dynamic_word`] does the folding
    /// through the *dominating* constant lattice, so a branch-conditional
    /// or otherwise unprovable binding still abstains and the written text
    /// is kept unchanged — the written text never resolves to anything, so
    /// abstaining is exactly the previous behaviour.
    ///
    /// A folded head that is still dynamic (a partially-resolved
    /// interpolation) is discarded rather than half-applied.
    ///
    /// A head that is a **whole-word** variable read (`$cmd`) is left alone:
    /// that shape belongs to the flow-sensitive const-dispatch engine
    /// (`pending_const_dispatches` ->
    /// [`super::diagnostics::const_dispatch`]), which settles it from the
    /// CFG/SSA value model rather than the walk's lexical map and marks the
    /// resulting invocation `indirect`.  Only the composite shapes that
    /// engine explicitly skips are folded here.
    /// The full command-resolution candidate list for a **folded** dynamic
    /// head, or an empty list when the head was written literally (the
    /// ordinary case, which `finalise_invocation_resolutions` rebuilds for
    /// itself).  See the call site for why the folded case cannot be
    /// rebuilt there.
    fn folded_head_candidates(
        &self,
        head: &str,
        folded: bool,
        scope_path: &[usize],
    ) -> Vec<String> {
        if !folded {
            return Vec::new();
        }
        let ns = self.command_resolution_namespace(scope_path);
        let path: Vec<&str> = self
            .namespace_paths
            .get(&ns)
            .map_or_else(Vec::new, |p| p.iter().map(String::as_str).collect());
        crate::naming::command_resolution_candidates(&ns, &path, head)
    }

    fn resolve_dynamic_command_head<'w>(
        &self,
        cmd_name: &'w str,
        cmd_tok: Token,
        head_is_single_token: bool,
        scope_path: &[usize],
    ) -> std::borrow::Cow<'w, str> {
        if !crate::naming::is_dynamic_word(cmd_name) || self.head_is_whole_word_variable(cmd_tok) {
            return std::borrow::Cow::Borrowed(cmd_name);
        }
        self.resolve_dynamic_word(cmd_name, Some(cmd_tok), head_is_single_token, scope_path)
            .filter(|folded| !crate::naming::is_dynamic_word(folded))
            .map_or(
                std::borrow::Cow::Borrowed(cmd_name),
                std::borrow::Cow::Owned,
            )
    }

    /// Whether the command head is a bare whole-word variable read (`$cmd`,
    /// `$ns::cmd`) rather than a composite (`${ns}::tail`) — the same
    /// truncate-at-the-brace test
    /// [`Self::record_var_or_cmd_command_site`] uses to decide which shape
    /// the const-dispatch settlement owns, so the two can never disagree
    /// about it.
    fn head_is_whole_word_variable(&self, cmd_tok: Token) -> bool {
        if cmd_tok.kind != TokenType::Var {
            return false;
        }
        let sm = Analyser::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        !sm.token_text(cmd_tok).contains('}')
    }

    /// Process a single segmented command.
    ///
    /// Walks `args` against every handler and stops at the first
    /// match. Non-matching commands fall through silently —
    /// they're either unknown (the W123 emitter handles reporting)
    /// or registry-known commands that don't need analyser-side
    /// intervention (the IR pass does the heavy lifting).
    ///
    /// `argv_texts` and `arg_tokens` parallel each other:
    /// `argv_texts[0]` is the command name, `argv_texts[1..]`
    /// the arguments. `arg_tokens[0]` is the command-name token.
    /// `single_token_word` is parallel to argv and indicates
    /// whether each word is a single atomic token (used by
    /// [`Self::bind_value_word_assignment`] for the const-string
    /// environment).
    ///
    /// Simple-command arity (E002 / E003) is emitted here via
    /// [`Self::emit_arity_diagnostics`]; the candidates are
    /// flushed post-walk by [`Self::flush_arity_diagnostics`].
    pub fn process_command(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        if argv_texts.is_empty() || arg_tokens_in.is_empty() {
            return;
        }
        #[cfg(debug_assertions)]
        let phase_started =
            std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").map(|_| std::time::Instant::now());
        #[cfg(debug_assertions)]
        let phase_site = arg_tokens_in[0].span.start();
        #[cfg(debug_assertions)]
        let phase_bytes = self.source.len();
        #[cfg(debug_assertions)]
        let report_phase = |stage: &str| {
            if let Some(started) = &phase_started {
                eprintln!(
                    "ANALYSER_COMMAND_PHASE bytes={phase_bytes} site={phase_site} stage={stage} ms={}",
                    started.elapsed().as_millis()
                );
            }
        };
        #[cfg(not(debug_assertions))]
        let report_phase = |_: &str| {};
        // See `AnalyserState::presubstituted_args`: taken (and reset) so only
        // *this* command level skips the substitution re-walks below.
        let presubstituted_args = std::mem::take(&mut self.presubstituted_args);
        if !presubstituted_args {
            self.retain_original_static_source_names(arg_tokens_in);
        }
        #[cfg(debug_assertions)]
        report_phase("original-names");
        let cmd_name = argv_texts[0].as_str();
        // Conditional source visibility never removes possible invocation
        // effects or workspace edges without an actual hidden-slot receipt.
        self.observe_interp_visibility(arg_tokens_in, scope_path);
        let args = &argv_texts[1..];
        let arg_tokens = &arg_tokens_in[1..];
        let arg_single = single_token_word.get(1..).unwrap_or_default();
        self.observe_indirect_interp_visibility(arg_tokens_in, scope_path);
        // Record this invocation so the post-walk
        // ``emit_unresolved_command_diagnostics`` (W123) can iterate
        // every command head the analyser visited.  ``inv.range``
        // anchors at the command-head token so the W123 message
        // points at the unresolved name rather than the whole
        // command line.
        let cmd_tok = arg_tokens_in[0];
        // Retain constructor reporting metadata in structure-only analyses
        // as well as full analyses. Source receiver consumers independently
        // require the positioned constructor or instance issuer; these maps
        // do not establish Native command selection or successful allocation.
        let creation_ns = self.command_resolution_namespace(scope_path);
        self.record_instance_creation(cmd_name, args, &creation_ns, cmd_tok.span.start());
        // Structure-only mode (item-tree extraction) skips every diagnostic /
        // cross-feature recording pass below — they don't affect the declared
        // proc / class / alias / ensemble structure and are the bulk of the
        // per-command cost.  The structural handlers further down still run, so
        // `file_decls` is identical to a full `analyse` (gated by the
        // `file_decls_corpus` corpus test).
        if !presubstituted_args {
            self.record_original_scoped_body_advice(cmd_tok.span.start(), arg_tokens, scope_path);
        }
        #[cfg(debug_assertions)]
        report_phase("scoped-bodies");
        self.record_original_variable_receivers(cmd_tok.span.start(), arg_tokens, scope_path);
        #[cfg(debug_assertions)]
        report_phase("variable-receivers");
        self.record_dynamic_variable_name_sites(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            cmd_tok.span.start(),
            presubstituted_args,
        );

        if !self.structure_only {
            self.record_direct_command_invocation(
                argv_texts,
                arg_tokens_in,
                single_token_word,
                arg_expand_in,
                scope_path,
            );

            // iRules ``call PROC ARG...`` — record an additional
            // ``CommandInvocation`` for the target proc so that
            // references, rename, and call-hierarchy see through the
            // indirection.
            self.record_irules_call_invocation(
                arg_tokens,
                scope_path,
                cmd_tok.span.start(),
                presubstituted_args,
            );

            #[cfg(debug_assertions)]
            report_phase("direct-invocations");
            self.record_command_site_facts(
                &DispatchSite {
                    cmd_name,
                    args,
                    arg_tokens,
                    arg_single,
                    arg_expand_in,
                    cmd_tok,
                    scope_path,
                    presubstituted_args,
                },
                arg_tokens_in,
                &report_phase,
            );
        } // end `if !self.structure_only`

        #[cfg(debug_assertions)]
        report_phase("dispatch-diagnostics");
        self.dispatch_command_handlers(
            cmd_name,
            args,
            arg_tokens,
            WordFacts { single: arg_single },
            cmd_tok,
            scope_path,
        );
        #[cfg(debug_assertions)]
        report_phase("handlers-complete");
    }

    fn record_command_site_facts(
        &mut self,
        site: &DispatchSite<'_>,
        arg_tokens_in: &[Token],
        report_phase: &impl Fn(&str),
    ) {
        let DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single: _,
            arg_expand_in,
            cmd_tok,
            scope_path,
            presubstituted_args,
        } = *site;
        // Walk every argument's source slice for ``[cmd ...]``
        // substitutions and record each nested head as its own
        // ``CommandInvocation``.
        if !presubstituted_args {
            self.record_nested_invocations_from_args(arg_tokens_in, scope_path);
        }

        report_phase("nested-invocations");
        // Record `ArgRole::CommandPrefix` callback heads (`lsort -command
        // myCompare`, `trace add … cb`) as command invocations too, so
        // find-references / rename / call-hierarchy / code-lens / W123 /
        // callback-arity see the callback exactly like a direct call.
        self.record_command_prefix_invocations(cmd_tok.span.start(), arg_tokens);

        report_phase("callback-invocations");
        // Record `ArgRole::CommandName` arguments (`info body PROC`,
        // `namespace which -command NAME`) — a bare command name held as
        // data — as command invocations too, so the named command is
        // reached by find-references / go-to-definition / rename without
        // any arity check (it is introspected, not called).
        self.record_command_name_invocations(args, arg_tokens, scope_path, cmd_tok.span.start());

        report_phase("command-name-invocations");
        // The occurrence tables the registry's *argument roles* produce —
        // namespace names and computed variable names.
        self.record_arg_role_facts(
            args,
            arg_tokens,
            scope_path,
            cmd_tok.span.start(),
            presubstituted_args,
        );

        report_phase("namespace-role-facts");
        // Run the per-command syntactic checks on commands nested inside
        // ``[…]`` substitutions (bare-`Cmd` args *and* braced-expr args)
        // — the main walk never descends a substitution (it treats
        // `[cmd …]` as a value), so a command like `set fh [open "|$cmd"
        // r]`, `set x [string index abc 99]` or `if { [matchclass …] }`
        // would otherwise escape the security / bounds / arity / style
        // families (IRULE2001/2002, W100, …) entirely.
        if !presubstituted_args {
            self.run_nested_command_diagnostics(arg_tokens_in, scope_path);
            self.run_nested_expr_diagnostics(site);
        }

        report_phase("nested-diagnostics");
        // Record variable-as-command and
        // command-substitution-as-command call sites so the
        // post-walk W307 / W308 emitters can resolve them.
        self.record_var_or_cmd_command_site(VarOrCmdSite {
            cmd_name,
            cmd_tok,
            head_expanded: arg_expand_in.first().copied().unwrap_or(false),
            args,
            arg_tokens,
            arg_expand: arg_expand_in.get(1..).unwrap_or(&[]),
            scope_path,
        });

        report_phase("variable-command-sites");
        // W125 (orphaned control-flow keyword) and IRULE5005 (direct
        // iRules-proc call without `call`) — both key off whether the
        // command head resolves to a user proc, so they share one
        // resolution.
        self.emit_proc_resolution_diagnostics(cmd_name, args, cmd_tok, scope_path);

        // When the constructor's class head is a `$var` reference
        // instead of a literal bareword, defer to the flow-sensitive
        // value model rather than dropping the
        // instance's class entirely.
        self.record_pending_instance_class_site(cmd_tok.span.start());

        report_phase("procedure-resolution");
        // Registry-owned expression arguments must be diagnosed before
        // body-owning handlers take their early-return paths.
        self.dispatch_expr_arguments(site);

        report_phase("expression-diagnostics");
        // Dispatch-site diagnostic emitters (W302 / W001 / E004 / W101
        // / W304 / W004 / E002-E003).  Extracted from this function so
        // it stays within the line budget; see the method for the
        // per-code rationale and ordering.  Run before the
        // early-returning handlers so option-bearing / body-owning
        // commands still get checked.
        self.emit_dispatch_site_diagnostics(site);
    }

    fn record_direct_command_invocation(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        let cmd_name = argv_texts[0].as_str();
        let args = &argv_texts[1..];
        let cmd_tok = arg_tokens_in[0];
        // A `${ns}::setdef`-shaped head is only *written* dynamically —
        // when `ns` is a constant that dominates this call the target is
        // as statically determined as a literal one, so resolution runs
        // on the folded name.  A head that cannot be
        // folded keeps its written text and resolves to nothing, exactly
        // as before.
        let head = self.resolve_dynamic_command_head(
            cmd_name,
            cmd_tok,
            single_token_word.first().copied().unwrap_or(false),
            scope_path,
        );
        let resolved = self.resolve_command_qualified_name(&head, scope_path);
        let arg_count = call_arg_count(args, arg_expand_in);
        // A folded head resolves to a real command, but its span is not
        // the written name — `${ns}::setdef` spells only the tail — so it
        // is a *reference*, never a rename target: overwriting the span
        // would splice the new name over the substitution itself.
        let folded = matches!(head, std::borrow::Cow::Owned(_));
        let computed = folded
            || !single_token_word.first().copied().unwrap_or(false)
            || !matches!(cmd_tok.kind, TokenType::Esc | TokenType::Str)
            || arg_expand_in.first().copied().unwrap_or(false);
        // A folded head's *written* name (`${ns}::setdef`) is not the
        // `{ns}::{name}` shape `finalise_invocation_resolutions` recovers
        // the calling namespace from, so that pass cannot rebuild this
        // call's candidate list — and without one it also cannot demote
        // the walk's local-first guess to the global candidate Tcl really
        // dispatches to.  `set ns tk; ${ns}::setdef …` inside `::tk` was
        // left pinned to the non-existent `::tk::tk::setdef`, so
        // find-references from `::tk::setdef`'s own declaration missed the
        // call while go-to-definition (which re-resolves from the cursor)
        // found it.  The folded name *is*
        // known here, so the list is built now and finalise settles
        // against it instead of rebuilding.  The `namespace path` read
        // here is the walk's current one: a path declared *later* in the
        // file is not retro-applied to a folded head (every other
        // walk-time resolution has the same horizon; finalise's own
        // rebuild is what normally widens it).
        let folded_candidates = self.folded_head_candidates(&head, folded, scope_path);
        let (original_name_input, original_lookup) = self
            .retained_invocation_tokens(cmd_tok.span.start(), arg_tokens_in)
            .and_then(|tokens| {
                let binding = tokens.source_binding.as_ref()?;
                let input = binding.original_head_name_input(&tokens)?;
                let lookup = binding.original_command_lookup(&tokens, &input);
                Some((Some(input), lookup))
            })
            .unwrap_or_default();
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_name_input,
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup,
                resolved_qualified_name: Some(resolved.clone()),
                resolution_candidates: folded_candidates,
                indirect: computed,
                rename_safe: !computed,
                ..crate::signature_scan::types::SignatureCommandInvocation::written(
                    cmd_name.to_owned(),
                    cmd_tok.span,
                    arg_count,
                )
            },
        );
        // `<ensemble> <subcommand> …` — record an additional, existence
        // -probed `CommandInvocation` for the subcommand word so
        // references, rename, call-hierarchy, and go-to-definition see
        // through a static `namespace ensemble create -map`/
        // `-subcommands` mapping the same way they already see through
        // an `interp alias`.
        self.record_ensemble_subcommand_invocation(&resolved, args, arg_tokens_in, arg_expand_in);
    }

    /// Reconstruct original lexical words once, then attach the retained
    /// lookup receipt. Consumers must not recover roles from final imports
    /// or a display head when this actual source site has no applicable shape.
    pub(super) fn retained_invocation_tokens(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Option<Arc<crate::ir::CommandTokens>> {
        // Implementation contract: naming.source.original-command-token-memo
        // docs/design/analysis/name-resolution-proofs/original-command-token-memo.md
        self.head_identities.retained_original_tokens(
            &tcl_lexer::SourceImage::document(&self.source),
            self.lexer_config(),
            invocation_offset,
            argument_tokens,
        )
    }

    fn retain_original_static_source_names(&mut self, argument_tokens: &[Token]) {
        let Some(first) = argument_tokens.first() else {
            return;
        };
        let Some(tokens) = self.retained_invocation_tokens(first.span.start(), argument_tokens)
        else {
            return;
        };
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.lexer_config();
        if !bindings.matches_original_source_image(&image, config)
            || origin.source_image() != &image
            || tokens.argv.len() != argument_tokens.len()
            || tokens
                .argv
                .iter()
                .zip(argument_tokens)
                .any(|(span, token)| *span != token.span)
        {
            return;
        }
        let Some(native) = crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            first.span.start(),
            config,
        ) else {
            return;
        };
        // Only the authoritative executable arena selects substitution
        // scripts. Inert braced text cannot become a lexical child command.
        // Iteration and exact original body spans keep the walk independent
        // of diagnostics, reached-parent lookup and entered runtime frames.
        let mut pending = vec![(native, None)];
        let mut visited = std::collections::HashSet::new();
        while let Some((native, body_origin)) = pending.pop() {
            // Re-entering the same lexical vector through an ordinary handler
            // walk must preserve its already sealed full source ancestry. This
            // is source membership only; no parent lookup is transplanted.
            let body_origin = body_origin.or_else(|| {
                let head = native.first()?;
                let occurrence = self
                    .result
                    .original_vendor_source_names
                    .get(&head.span())?
                    .as_ref()?;
                (occurrence.original_words() == native.as_slice()
                    && occurrence.name_input().original_word() == head
                    && occurrence.site().source.source_image() == &image
                    && occurrence.name_input().lexer_config() == config)
                    .then(|| occurrence.retained_body_origin())
                    .flatten()
            });
            self.retain_original_static_source_vector(&native, body_origin.as_ref());
            if let Some(words) =
                self.original_readonly_vendor_source_vector(&native, body_origin.as_ref())
            {
                let context = self.analysis_context();
                for body in words.source_script_bodies_for(
                    &context,
                    crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
                ) {
                    if !body.matches_source(&image, config)
                        || !body.matches_context(&context)
                        || !visited.insert(body.content_span())
                    {
                        continue;
                    }
                    let Some(origin) = body.vendor_origin() else {
                        continue;
                    };
                    if let Ok(plan) = tcl_lexer::native_script_words_in(
                        image.clone(),
                        body.content_span(),
                        config,
                    ) {
                        pending.extend(
                            plan.commands
                                .into_iter()
                                .map(|command| (command.words, Some(origin.clone()))),
                        );
                    }
                }
            }
            for word in &native {
                for part in word.executable_parts().all_parts() {
                    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
                        continue;
                    };
                    if !visited.insert(body) {
                        continue;
                    }
                    let Ok(plan) = tcl_lexer::native_script_words_in(image.clone(), body, config)
                    else {
                        continue;
                    };
                    // A malformed tail supplies no complete vector; any
                    // independently complete prefix is still readonly syntax.
                    pending.extend(
                        plan.commands
                            .into_iter()
                            .map(|command| (command.words, body_origin.clone())),
                    );
                }
            }
        }
    }

    /// Seal authored hosted roles for a genuine lexical vector. Unreached
    /// script children may retain source applicability, never parent lookup.
    fn original_readonly_vendor_source_vector(
        &self,
        native: &[tcl_lexer::NativeWord],
        body_origin: Option<
            &std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>,
        >,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let first = native.first()?;
        let last = native.last()?;
        let config = self.lexer_config();
        let text = self
            .source
            .get(first.span().start() as usize..last.span().end() as usize)?;
        let mut commands = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            first.span().start(),
            config,
        );
        if commands.len() != 1 {
            return None;
        }
        let segment = commands.pop()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(&self.source),
            config,
            &segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        let site = crate::command_binding::CommandAllocationSite {
            source: std::sync::Arc::clone(
                self.head_identities.source_bindings_ref().source_origin()?,
            ),
            offset: first.span().start(),
        };
        let occurrence = crate::signature_scan::vendor_name::VendorSourceNameOccurrence::new(
            &site,
            first,
            self.vendor_source_name_policy()?,
            std::sync::Arc::from(native),
        )?
        .with_body_origin(body_origin);
        let context = self.analysis_context();
        let metadata = crate::registry_invocation::original_vendor_occurrence_registry_metadata(
            &context,
            &tokens,
            &occurrence,
        );
        crate::registry_invocation::source_structure::original_vendor_registry_words(
            &context, &tokens, metadata?,
        )
    }

    fn retain_original_static_source_vector(
        &mut self,
        native: &[tcl_lexer::NativeWord],
        body_origin: Option<
            &std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>,
        >,
    ) {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.lexer_config();
        if !bindings.matches_original_source_image(&image, config)
            || origin.source_image() != &image
            || native
                .iter()
                .any(|word| word.image() != &image || word.config() != config)
        {
            return;
        }
        let Some(head) = native.first() else {
            return;
        };
        // This site retains the complete lexical source vector only. A
        // reached invocation, selected role and installation are independent.
        let site = crate::command_binding::CommandAllocationSite {
            source: std::sync::Arc::clone(origin),
            offset: head.group().span.start(),
        };
        if let Some(policy) = self.vendor_source_name_policy() {
            let original_words: std::sync::Arc<[tcl_lexer::NativeWord]> =
                std::sync::Arc::from(native);
            for word in native {
                let Some(occurrence) =
                    crate::signature_scan::vendor_name::VendorSourceNameOccurrence::new(
                        &site,
                        word,
                        policy,
                        std::sync::Arc::clone(&original_words),
                    )
                else {
                    continue;
                };
                let occurrence = occurrence.with_body_origin(body_origin);
                for span in [
                    Some(word.span()),
                    word.tokens().first().map(|token| token.span),
                ]
                .into_iter()
                .flatten()
                {
                    match self.result.original_vendor_source_names.entry(span) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(Some(occurrence.clone()));
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            let combined = entry
                                .get()
                                .as_ref()
                                .and_then(|existing| existing.merge_source_origin(&occurrence));
                            entry.insert(combined);
                        }
                    }
                }
            }
        }
        let Some(policy) = self.declaration_name_policy() else {
            return;
        };
        let rules = self.word_rules();
        for word in native {
            let Some(key) =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    word, rules, policy,
                )
            else {
                continue;
            };
            let Some(occurrence) =
                crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(&site, key)
            else {
                continue;
            };
            for span in [
                Some(word.span()),
                word.tokens().first().map(|token| token.span),
            ]
            .into_iter()
            .flatten()
            {
                match self.original_static_source_names.entry(span) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(Some(occurrence.clone()));
                    }
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        if entry.get().as_ref() != Some(&occurrence) {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn vendor_source_name_policy(
        &self,
    ) -> Option<tcl_syntax::naming::VendorSourceNamePolicy> {
        let policy = if let Some(policy) = self
            .resolved_input
            .as_ref()
            .and_then(super::input::ResolvedAnalysisInput::vendor_source_policy)
        {
            policy
        } else {
            let generation = self.analysis_context();
            let context = tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &generation.context().environment.id)?;
            tcl_syntax::naming::VendorSourceNamePolicy::authored(context)?
        };
        match self
            .source_analysis_entry
            .as_deref()
            .and_then(|entry| entry.execution_name_policy)
        {
            Some(tcl_syntax::naming::ExecutionNamePolicy::ObservedBigIp(observed)) => (policy
                .context()
                == tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::TmmIRule)
                .then(|| tcl_syntax::naming::VendorSourceNamePolicy::observed(observed)),
            _ => Some(policy),
        }
    }

    /// Original static syntax occurrence captured from this walk's exact
    /// complete command vector. It supplies no Registry role or execution.
    pub(super) fn original_static_source_name_at_span(
        &self,
        span: Span,
    ) -> Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        self.original_static_source_names.get(&span)?.as_ref()
    }

    fn retained_argument_role_assistance(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Vec<(usize, ArgRole)> {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return Vec::new();
        };
        let mut roles = crate::registry_invocation::invocation_argument_role_assistance(
            registry,
            generation.as_ref(),
            &self.command_surface(registry),
            &tokens,
        );
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        let context = generation;
        if let Some(advice) = self
            .head_identities
            .original_source_transition_advice(&context, &tokens)
            && let Some(words) =
                crate::registry_invocation::source_structure::source_transition_words_from_advice(
                    &self.source,
                    self.lexer_config(),
                    &context,
                    advice,
                )
        {
            for role in words.written_argument_roles() {
                if !roles.contains(&role) {
                    roles.push(role);
                }
            }
        }
        roles.sort_by_key(|(ordinal, _)| *ordinal);
        roles
    }

    fn retained_argument_role_consensus(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Vec<(usize, ArgRole)> {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return Vec::new();
        };
        crate::registry_invocation::invocation_argument_role_consensus(
            registry,
            self.analysis_context().as_ref(),
            &tokens,
        )
    }

    /// Run E006 for the argument shapes the active command spec identifies as
    /// formal lists and static-variable lists. This is deliberately a
    /// original Registry/declaration query, including resolver-defined roles,
    /// effective argv lineage and nested lambda literals. Known replacements
    /// cannot borrow a nominal builtin formal-list role.
    fn emit_formal_parameter_list_diagnostics(
        &mut self,
        original: Option<&super::diagnostic_registry::OriginalDiagnosticSource>,
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
        super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            self,
            args,
            arg_tokens,
            &parameter_indices,
        );
        let static_indices = indices(ArgRole::StaticVarList);
        super::diagnostics::emit_invalid_static_variable_list_diagnostics(
            self,
            args,
            arg_tokens,
            &static_indices,
        );
        let lambda_indices = indices(ArgRole::LambdaLiteral);
        super::diagnostics::emit_invalid_lambda_parameter_list_diagnostics(
            self,
            args,
            arg_tokens,
            &lambda_indices,
        );
    }

    /// Record an iRules `call PROC ARG...`'s target as its own
    /// `CommandInvocation` so references / rename / call-hierarchy see
    /// through the indirection. Extracted from [`Self::process_command`]
    /// so it stays within the line budget.
    fn record_irules_call_invocation(
        &mut self,
        argument_tokens: &[Token],
        scope_path: &[usize],
        offset: u32,
        presubstituted: bool,
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let Some(original) = self.original_name_source(offset, argument_tokens, presubstituted)
        else {
            return;
        };
        let Some(registry) = original.registry() else {
            return;
        };
        let Some(argument) = registry
            .with_schema(selected_rule_procedure_operand)
            .flatten()
        else {
            return;
        };
        let Some(target_name) = registry.literal(argument).map(str::to_owned) else {
            return;
        };
        let Some(word) = original.word(argument) else {
            return;
        };
        let span = word.span();
        let original_name_input = registry
            .words()
            .operands()
            .get(argument)
            .and_then(Option::as_ref)
            .and_then(|operand| operand.input())
            .cloned();
        let resolved = self.resolve_command_qualified_name(&target_name, scope_path);
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input,
                lookup:
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        invocation_offset: offset,
                    },
                name: target_name,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc: None,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                // A selected source schema supplies a candidate reference,
                // not a current owning rule or writable command identity.
                rename_safe: false,
                existence_probe: false,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            },
        );
    }

    /// Typed per-command dispatch for [`Self::process_command`].
    ///
    /// Which handler family owns a command form is registry data — the
    /// [`AnalyserHookId`] stamped on the `CommandSpec` (or, for the
    /// subcommand-shaped families like `namespace eval` / `dict for` /
    /// `interp alias`, on the `SubCommand`) — so the dispatch is one
    /// typed `match`, not a chain of name-guarded calls.  The
    /// early-return families stop the walk by returning `true`; the void
    /// families fall through to the shared
    /// tail: the registry-role-driven handlers that consider every
    /// command (`VarWrite` bindings, symbol definers, the tcllib
    /// `::import` wrapper idiom) and the generic `ArgRole::Body`
    /// recursion.
    fn dispatch_command_handlers(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        // IRULE5001's debug gate spans everything below: the hook handlers
        // that own their own body walk (`switch`, `foreach`, `catch`) and the
        // generic `ArgRole::Body` recursion (`if`, `while`, `for`, `try`)
        // alike. Bracketing the whole dispatch is what makes nested bodies
        // inherit the gate.
        let gated = self.irules_debug_gate_opens(cmd_name, args, arg_tokens, cmd_tok);
        if gated {
            self.irules_debug_gate_depth += 1;
        }
        self.dispatch_command_handlers_inner(
            cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
        );
        if gated {
            self.irules_debug_gate_depth -= 1;
        }
    }

    /// The dispatch itself — see [`Self::dispatch_command_handlers`], which
    /// wraps this in IRULE5001's debug-gate bracket.
    fn dispatch_command_handlers_inner(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        let arg_single = words.single;
        if self.dispatch_analyser_hook(cmd_name, args, arg_tokens, words, cmd_tok, scope_path) {
            return;
        }

        // The scope aliases the call's state transitions state — `global`,
        // `variable`, `upvar`, `namespace upvar`, a pack command's alias
        // facts — resolved over the words' source facts, so a computed word
        // reaches the registry resolver as computed.
        self.apply_invocation_transitions(cmd_name, args, arg_tokens, words, cmd_tok, scope_path);
        // The loop and output variables the call's roles name (`foreach`'s
        // var lists, `dict for`'s pair, `lassign`, `scan`, `regexp`, `incr`,
        // `append`, …), so completion/hover/definition see the bound names.
        self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
        // A direct one-target write of a value word (`set name value`) binds
        // its name to that word — the constant-string environment, a
        // created interpreter's key and the search-path record — from the
        // registry's `CellWrite` declaration, not the command's spelling.
        self.bind_value_word_assignment(cmd_tok, arg_tokens, scope_path);
        // Registry symbol-definer commands (`tcltest::test NAME …`) contribute a
        // lightweight named definition to the outline.  Void handler — it only
        // records the symbol; the body still recurses via the generic
        // `ArgRole::Body` walk below.
        // Definition and body descriptors are registry semantics, so read the
        // document's offset-keyed identity before asking either one.  This is
        // what lets an iRules event handler reached through a proven alias or
        // rename contribute the same outline/context as `when`, while a
        // spelling a user `proc` took over cannot inherit `when`'s body.
        let generation = self.analysis_context();
        let symbol_advice = (|| {
            let tokens = self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens)?;
            if self.result.has_original_vendor_source_names() {
                let image = tcl_lexer::SourceImage::document(&self.source);
                let config = self.lexer_config();
                let original = self.result.original_vendor_source_name_in_source(
                    &image,
                    config,
                    cmd_tok.span,
                )?;
                let words = crate::registry_invocation::original_native_compiler_words(
                    &image,
                    tokens.words(),
                    cmd_tok.span.start(),
                    config,
                )?;
                return crate::registry_invocation::vendor_symbol_declaration_advice(
                    &generation,
                    &tokens,
                    original.name_input(),
                    &words,
                );
            }
            crate::registry_invocation::original_symbol_declaration_advice(
                generation.commands(),
                generation.as_ref(),
                &tokens,
            )
            .or_else(|| {
                let head = self.original_static_source_name_at_span(cmd_tok.span)?;
                let namespace = super::scope::scope_at(&self.result.global_scope, scope_path)
                    .and_then(|scope| scope.naming_scope.as_ref());
                let metadata = crate::registry_invocation::original_conditional_registry_metadata(
                    &generation,
                    &tokens,
                    head,
                    namespace,
                )?;
                let symbol = metadata.symbol_definition()?;
                let supplied = metadata.original_words().len().checked_sub(1)?;
                if usize::from(symbol.name_arg) >= supplied
                    || symbol
                        .requires_arg
                        .is_some_and(|ordinal| usize::from(ordinal) >= supplied)
                {
                    return None;
                }
                Some(
                    crate::registry_invocation::OriginalSymbolDeclarationAdvice {
                        command: metadata.command().to_owned(),
                        symbol,
                        traits: metadata.possible_traits(),
                        conditional_metadata: Some(metadata),
                    },
                )
            })
        })();
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_SYMBOL_ADVICE").is_some() {
            eprintln!(
                "ORIGINAL_SYMBOL_ADVICE consumer offset={} tokens={} advice={} logical={}",
                cmd_tok.span.start(),
                self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens)
                    .is_some(),
                symbol_advice.is_some(),
                self.result.allows_lexical_declaration_advice()
            );
        }
        if let Some(advice) = symbol_advice {
            self.handle_defines_symbol(&advice, args, arg_tokens, arg_single, cmd_tok, scope_path);
        }
        // The tcllib `<NS>::import <alias>` wrapper idiom — recognised by
        // the call's own `::import` tail, not a registry name.
        self.handle_tcllib_import_wrapper(cmd_name, cmd_tok, args, scope_path);

        // Generic body recursion via the command registry's
        // `ArgRole::Body`.  Picks up `if` / `while` / `for` / `when` /
        // `eval` / `uplevel` / `subst` / etc. — every command whose
        // registry spec marks an argument index as `BODY`, each body
        // walked at the depth its clause timing (or, without a clause
        // grammar, the command's traits) gives it.  The early-return hook
        // arms above already consumed the commands that own their body
        // walk (proc, oo::class, oo::define, namespace eval, foreach,
        // switch, catch), so this loop only fires for the rest — `try`
        // among them since its hook retired.
        //
        // For `when EVENT { body }` the iRules dialect spec
        // marks arg 1 as BODY; set `current_event` for the body
        // walk so race-detection diagnostics see the event name.
        self.dispatch_body_arguments(
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            cmd_tok.span.start(),
            scope_path,
        );
    }

    /// Resolve the [`AnalyserHookId`] for a command head through the shared
    /// registry/context owner. A leading global qualifier on a rootable
    /// command (`::proc`, `::set`, `::namespace`, …) denotes the same exact
    /// registry binding as its bare spelling; method-context-only commands
    /// without a global form are rejected by the registry's rooted-fallback
    /// policy. Subcommand selection remains exact.
    ///
    /// Outside an `analyse*` run (unit harnesses drive handlers on a
    /// bare `Analyser::new()`) the shared core registry stands in for
    /// the stashed dialect-aware one.
    pub(in crate::analyser) fn resolve_analyser_hook(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<tcl_registry::hooks::AnalyserHookId> {
        self.resolve_analyser_hook_call(cmd_name, args)
            .map(|resolved| resolved.hook)
    }

    /// Declared transitions and their operands from the genuine selected source
    /// call. Captured alias operands keep their original producers and spans.
    fn original_registry_state_transitions(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) -> Option<(tcl_registry::StateTransitions, Vec<String>, Vec<Token>)> {
        let site = DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single: words.single,
            arg_expand_in: &[],
            cmd_tok,
            scope_path,
            presubstituted_args: false,
        };
        let original = self.original_diagnostic_invocation(&site)?;
        let transitions = original.with_schema(|schema| schema.state_transitions())?;
        let mut values = Vec::new();
        let mut tokens = Vec::new();
        for index in 0..original.words().arguments().len() {
            let word = original.word(index)?;
            tokens.push(*word.tokens().first()?);
            values.push(original.literal(index).unwrap_or("").to_owned());
        }
        Some((transitions, values, tokens))
    }

    fn apply_invocation_transitions(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if let Some((transitions, values, tokens)) = self.original_registry_state_transitions(
            cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
        ) {
            self.apply_state_transitions(&transitions, &values, &tokens, scope_path);
        }
    }

    /// Note, in the per-item shell walk, a definer only the workspace's packs
    /// declare: the shell reads the un-overlaid store, where `cmd_name` has no
    /// definition-body grammar, while the walk's own generation (the overlaid
    /// one) gives it one. The per-item analysis then takes the full path
    /// ([`super::per_item::PerItemFallback::PackDefiner`]), so the class the
    /// definer makes is not lost.
    fn note_pack_definer(&mut self, cmd_name: &str) {
        if self.defer_proc_bodies
            && self.pack_overlay != 0
            && self.definition_grammar(cmd_name).is_none()
            && self
                .analysis_context()
                .commands()
                .get(cmd_name)
                .is_some_and(|spec| spec.definition_body.is_some())
        {
            self.pack_definer_seen = true;
        }
    }

    /// [`Self::resolve_analyser_hook`] plus the traits and the clause plan of
    /// the concrete spec / subcommand the head resolved to — one resolution,
    /// every fact.
    ///
    /// A handler reached through hook dispatch must read its command's traits
    /// and clause structure from *this* resolution rather than re-fetching a
    /// spec by literal name: the name test puts per-command knowledge back in
    /// the analyser, and it silently diverges the moment a dialect variant (or
    /// a subcommand) shares the hook. Traits are composed
    /// `spec.traits | sub.traits`, matching
    /// [`tcl_registry::CommandRegistry::invocation_traits`].
    pub(super) fn resolve_analyser_hook_call(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<ResolvedAnalyserHook> {
        let generation = self.analysis_context();
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Select metadata under the complete retained authoring context.
        // This supplies neither head identity nor handler applicability.
        let resolved = tcl_registry::model::resolve_call_in_context(
            generation.commands(),
            Some(generation.context()),
            cmd_name,
            &arg_strs,
        )?;
        Some(ResolvedAnalyserHook {
            hook: resolved.analyser_hook?,
            traits: resolved.spec.traits
                | resolved
                    .sub
                    .map_or_else(tcl_registry::Traits::empty, |sub| sub.traits),
        })
    }

    /// The single typed `match` over the resolved [`AnalyserHookId`].
    ///
    /// Returns `true` when the command was consumed by an early-return
    /// family (the caller stops, skipping the shared tail), `false`
    /// when the walk should continue — either a void family ran, or no
    /// hook (and no definition-grammar definer) matched.
    ///
    /// `for`, `try`, `dict for`, `dict update`, `incr`, `append`, `lappend`,
    /// `upvar`, `namespace upvar`, `global` and `variable` carry no stamp at
    /// all: their only command-specific knowledge is a position or a
    /// keyword a descriptor states, so they take the
    /// "no stamped family" branch above and fall straight through to the
    /// shared tail below like any other command with no hook — the generic
    /// body walk reads when each body runs from its clause plan (`for`'s
    /// `start` once, `next` and the body per iteration; `try`'s handler
    /// bodies `Selected`), `apply_invocation_transitions` resolves a scope
    /// alias as the invocation's `VariableCellAliasTransition`, and
    /// `handle_var_binding_command` binds a loop or bound variable — with
    /// `lappend auto_path DIR…`'s record, the list append's
    /// `var_elements_effect` states — from its `LoopVarList` / `VarWrite`
    /// role, and the generic body walk binds the variable lists a clause
    /// fills (`try`'s handler variables, a slot the flat roles leave out).
    #[allow(
        clippy::too_many_lines,
        reason = "exhaustive registry-hook dispatch (one arm per AnalyserHookId \
                  variant, so a new variant is a compile error until wired); \
                  splitting hurts readability, per classify_side_effects"
    )]
    fn dispatch_analyser_hook(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        words: WordFacts<'_>,
        cmd_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        use tcl_registry::hooks::AnalyserHookId as Hook;
        let arg_single = words.single;
        if let Some(handled) = self.dispatch_original_class_definer(cmd_tok, scope_path) {
            return handled;
        }
        if let Some(handled) = self.dispatch_original_class_configuration(cmd_tok, scope_path) {
            return handled;
        }
        let Some(ResolvedAnalyserHook { hook, .. }) =
            self.resolve_analyser_hook_call(cmd_name, args)
        else {
            // No stamped family — the definition-grammar-driven definers
            // (TclOO metaclass create, snit::type/widget, itcl::class) get
            // their chance, then the tracked-interpreter-handle dispatch
            // (`sandbox eval { … }`, `cmd_name` never a registry name so it
            // can never reach a stamped hook the way literal `interp eval`
            // does).  Their registry/analysis-state conditions are disjoint
            // from every stamped hook and from each other (pinned by
            // tcl-registry's analyser-hook drift tests for the registry
            // ones), so running them only on the hookless path preserves the
            // dispatch order.
            self.note_pack_definer(cmd_name);
            return self.handle_oo_class_command(cmd_name, args, arg_tokens, scope_path, cmd_tok)
                || self.handle_snit_type_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_itcl_class_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_jim_class_command(cmd_name, args, arg_tokens, scope_path)
                || self
                    .handle_jim_class_member_call(cmd_name, args, arg_tokens, scope_path, cmd_tok)
                || self.handle_interp_handle_eval_command_original(
                    cmd_name,
                    args,
                    arg_tokens,
                    scope_path,
                    Some(cmd_tok),
                );
        };
        match hook {
            // Early-return families: the handler owns the whole command
            // (including its body walk) when it returns `true`.
            Hook::Proc => {
                self.handle_proc_command(cmd_name, args, arg_tokens, arg_single, scope_path)
            }
            Hook::OptProc => self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path),
            // `interp eval path { … }` — the child interpreter's script is
            // analysed in an isolated scope; a `{}`/multi-word/dynamic shape
            // falls through to the generic body walk in the current scope.
            Hook::InterpEval => self.handle_interp_eval_command_original(
                args,
                arg_tokens,
                scope_path,
                Some(cmd_tok),
            ),
            Hook::OoDefine => {
                self.handle_oo_define_command(cmd_name, args, arg_tokens, arg_single, scope_path)
            }
            Hook::NamespaceEval => self.handle_namespace_eval_command_at_invocation(
                args,
                arg_tokens,
                arg_single,
                scope_path,
                Some(cmd_tok.span.start()),
            ),
            // uplevel #0 { body } — opens a global-frame child scope so
            // the body's locals don't leak into the enclosing proc's
            // variable set.  Only the `#0` form is consumed; other
            // levels fall through to the generic body recursion.
            Hook::Uplevel => self.handle_uplevel_command(args, arg_tokens, scope_path),
            Hook::Foreach => {
                self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
                self.handle_foreach_command(cmd_name, args, arg_tokens, scope_path)
            }
            Hook::Switch => self.handle_switch_command(cmd_name, args, arg_tokens, scope_path),
            Hook::Catch => self.handle_catch_command(args, arg_tokens, scope_path),
            // apply {{params} body} — owns its body walk (binds params,
            // analyses element 1) so the generic `ArgRole::Body`
            // recursion never mis-reads the parameter list as a command.
            Hook::Apply => {
                let original = self.original_interp_apply_body(cmd_tok, arg_tokens, scope_path);
                self.handle_apply_command_with_source_namespace(
                    args, arg_tokens, scope_path, original,
                )
            }

            // Void families: run the handler(s), then fall through to
            // the shared tail.
            Hook::InterpCreate => {
                if let Some((transitions, _, _)) = self.original_registry_state_transitions(
                    cmd_name, args, arg_tokens, words, cmd_tok, scope_path,
                ) {
                    self.handle_interp_create_command(&transitions);
                }
                if let Some(path) = args
                    .get(1..)
                    .and_then(super::handlers::parse_interp_create_path)
                {
                    self.retain_interp_visibility_declaration(path, cmd_tok, arg_tokens);
                }
                false
            }
            Hook::InterpDelete => {
                self.handle_interp_delete_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::InterpHide => {
                self.handle_interp_hide_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::InterpExpose => {
                self.handle_interp_expose_command_original(args, cmd_tok, arg_tokens);
                false
            }
            Hook::DictWith => {
                self.handle_dict_with_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceEnsemble => {
                self.handle_namespace_ensemble(args, arg_tokens, scope_path);
                false
            }
            Hook::InterpAlias => {
                self.retain_interp_visibility_alias(cmd_tok, arg_tokens);
                self.handle_interp_alias(args, scope_path, cmd_tok.span.start());
                false
            }
            Hook::OoObjdefine => self.handle_oo_objdefine(args, arg_tokens, arg_single, scope_path),
            Hook::PackageRequire => {
                self.handle_package_require(cmd_name, cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackageProvide => {
                self.handle_package_provide(cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackageIfneeded => {
                self.handle_package_ifneeded(cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackagePrefer => {
                self.handle_package_prefer(cmd_tok, args);
                false
            }
            Hook::Source => {
                self.handle_source_command(args, arg_tokens, arg_single, scope_path);
                false
            }
            Hook::NamespaceImport => {
                self.handle_namespace_import_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceExport => {
                self.handle_namespace_export_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceForget => {
                self.handle_namespace_forget_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespacePath => {
                self.handle_namespace_path_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceUnknown => {
                self.handle_namespace_unknown_command(args);
                false
            }
            Hook::RegexPatternCapture => {
                self.handle_regex_pattern_capture(cmd_name, args, arg_tokens, scope_path, cmd_tok);
                false
            }
            // ``load`` unconditionally flips ``has_dynamic_providers``:
            // it brings a shared library's commands into the interpreter
            // at runtime, which static W123 unknown-command analysis can
            // never predict.
            Hook::Load => {
                self.result.has_dynamic_providers = true;
                false
            }
            // A *static* ``rename OLD NEW`` is recorded precisely by
            // ``handle_rename`` (``NEW`` resolves to whatever ``OLD``
            // denoted, including its arity) — only a genuinely *dynamic*
            // rename (``rename $x y`` / ``rename x [y]``) falls back to
            // the same conservative flag as ``load``, matching
            // ``command_binding.rs``'s wildcard-collapse convention for
            // the identical shape.
            Hook::Rename => {
                if self.handle_rename(
                    args,
                    arg_tokens,
                    arg_single,
                    scope_path,
                    cmd_tok.span.start(),
                ) {
                    self.result.has_dynamic_providers = true;
                }
                false
            }
        }
    }

    /// Emit the two diagnostics that key off whether the command head
    /// resolves to a user proc:
    ///
    /// - **W125** (Warning) — an orphaned control-flow keyword
    ///   (`else` / `elseif` / `then` / `on` / `trap` / `finally`) used as a
    ///   standalone command.  This almost always means a misplaced newline
    ///   split its parent `if` / `try` (`}\nelse {` instead of `} else {`).
    ///   Suppressed when a user proc *or* a registry command of the same
    ///   name shadows the keyword.
    /// - **IRULE5005** (Error) — a user proc invoked directly inside an
    ///   iRules event body, where Tcl-on-TMM requires `call PROC ARGS`.
    ///   Ships a `call PROC`-rewrite [`CodeFix`].
    ///
    /// The proc is resolved once and shared between both checks.
    pub(super) fn emit_proc_resolution_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        let resolves_to_proc = self.resolve_proc_call(cmd_name, scope_path).is_some();

        if !resolves_to_proc
            && let Some(parent) = tcl_registry::clause_grammar::owner_of_keyword(cmd_name)
            && self
                .analysis_context()
                .context()
                .resolve_spec(self.analysis_context().commands(), cmd_name)
                .is_none()
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
        if resolves_to_proc && irules_proc_dispatch_context {
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

    /// The script-injection family — a script or command line built by
    /// concatenating substituted words, which the interpreter then re-parses.
    ///
    /// Grouped out of [`Self::emit_dispatch_site_diagnostics`] because they
    /// are one subject with one shared argument shape, and because that
    /// dispatcher is at its line budget.
    fn emit_injection_diagnostics(
        &mut self,
        site: &DispatchSite<'_>,
        original: Option<&super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            cmd_tok,
            ..
        } = *site;
        self.emit_w101_eval_string_concat(original);
        self.emit_w102_subst_injection(original);
        self.emit_w103_open_pipeline(cmd_name, args, arg_tokens, arg_single);
        self.emit_w300_source_variable(cmd_name, args, arg_tokens);
        self.emit_w309_eval_subst_double_decode(original);
        self.emit_w301_uplevel_injection(original);
        self.emit_w312_interp_eval_injection(original);
        self.emit_w303_redos(cmd_name, args, arg_tokens, cmd_tok);
    }

    /// Registry-owned literal/value diagnostics. Kept as one dispatch-site
    /// group so adding a registry validator does not grow the main diagnostic
    /// dispatcher; none of these consumers knows a command name.
    fn emit_registry_argument_diagnostics(
        &mut self,
        _site: &DispatchSite<'_>,
        original: Option<&super::diagnostic_registry::OriginalDiagnosticInvocation>,
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
        original: Option<&super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let generation = self.analysis_context();
        let registry = super::bounds_checks::BoundsMetadataContext::Retained(&generation);
        let grammar = self.grammar();
        let surface = self.command_surface(generation.commands());
        if let Some(candidate) = super::bounds_checks::loop_termination_candidate(
            cmd_name,
            args,
            arg_tokens,
            registry,
            Some(&surface),
            self.lexer_config(),
            &grammar,
        ) {
            self.loop_candidates.push(candidate);
        }
        let numbers = grammar.numbers;
        let idx_diags =
            original.map_or_else(Vec::new, super::bounds_checks::original_index_diagnostics);
        let lset_diags = super::bounds_checks::lset_index_diagnostics(
            cmd_name,
            args,
            arg_tokens,
            &self.source,
            registry,
            self.lexer_config(),
            numbers,
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
    fn emit_dispatch_site_diagnostics(&mut self, site: &DispatchSite<'_>) {
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
            None
        } else {
            self.retained_invocation_tokens(cmd_tok.span.start(), arg_tokens)
                .and_then(|tokens| {
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
        self.emit_injection_diagnostics(site, original.as_ref());
        self.emit_w306_literal_expected(original.as_ref());
        // W310 runs for every command (it scans args for credential
        // option flags), so it takes no cmd_name guard.
        self.emit_w310_hardcoded_credentials(cmd_name, args, arg_tokens);
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
        self.emit_w311_encoding_mismatch(cmd_name, args, arg_tokens);
        self.emit_binary_field_version_gates(&format_templates);
        self.emit_w121_invalid_subnet_mask(args, arg_tokens);
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
            self.record_proven_site(&super::diagnostics::CallWords {
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
        original: Option<&super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        self.emit_arity_diagnostics(
            site.cmd_name,
            &super::diagnostics::ArityWords {
                args: site.args,
                arg_tokens: site.arg_tokens,
                arg_expand: site.arg_expand_in.get(1..).unwrap_or(&[]),
                cmd_tok: site.cmd_tok,
            },
            site.scope_path,
            original,
        );
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

    fn original_diagnostic_segment(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<(SegmentedCommand, tcl_lexer::NativeWord)> {
        self.original_source_segment(
            site.cmd_tok.span.start(),
            site.arg_tokens,
            site.presubstituted_args,
        )
    }

    /// Complete source geometry is shared by naming producers and diagnostics.
    /// A produced argv never acquires the source vector of its building call.
    fn original_source_segment(
        &self,
        offset: u32,
        argument_tokens: &[Token],
        presubstituted: bool,
    ) -> Option<(SegmentedCommand, tcl_lexer::NativeWord)> {
        if presubstituted {
            return None;
        }
        let tokens = self.retained_invocation_tokens(offset, argument_tokens)?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.lexer_config();
        let native = crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            offset,
            config,
        )?;
        let first = native.first()?;
        let last = native.last()?;
        let source = self
            .source
            .get(first.span().start() as usize..last.span().end() as usize)?;
        let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            first.span().start(),
            config,
        );
        if segments.len() != 1 {
            return None;
        }
        let segment = segments.pop()?;
        Some((segment, first.clone()))
    }

    /// Selected original Registry or declared syntax under the actual input.
    /// This is a source naming purpose; it supplies no successful dispatch.
    fn original_name_source(
        &self,
        offset: u32,
        argument_tokens: &[Token],
        presubstituted: bool,
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticSource> {
        use super::diagnostic_registry::{OriginalDiagnosticInvocation, OriginalDiagnosticSource};
        let (segment, first) =
            self.original_source_segment(offset, argument_tokens, presubstituted)?;
        if let Some(words) = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        ) {
            if words.head_source()?.word()? != &first {
                return None;
            }
            return OriginalDiagnosticInvocation::new(words, self.analysis_context())
                .map(OriginalDiagnosticSource::Registry);
        }
        let declared = crate::registry_invocation::source_structure::source_declared_command_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        (declared.original_words().first()? == &first)
            .then(|| OriginalDiagnosticSource::Declared(Arc::new(declared)))
    }

    /// Selected original Registry source syntax under the actual full context.
    fn original_diagnostic_invocation(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticInvocation> {
        let (segment, first) = self.original_diagnostic_segment(site)?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        if words.head_source()?.word()? != &first {
            return None;
        }
        super::diagnostic_registry::OriginalDiagnosticInvocation::new(
            words,
            self.analysis_context(),
        )
    }

    pub(in crate::analyser) fn original_diagnostic_call_at(
        &self,
        cmd_tok: Token,
        argument_tokens: &[Token],
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticInvocation> {
        match self.original_name_source(cmd_tok.span.start(), argument_tokens, false)? {
            super::diagnostic_registry::OriginalDiagnosticSource::Registry(original) => {
                Some(original)
            }
            super::diagnostic_registry::OriginalDiagnosticSource::Declared(_) => None,
        }
    }

    pub(in crate::analyser) fn original_substitution_call(
        &self,
        word: &tcl_lexer::NativeWord,
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticInvocation> {
        let [token] = word.tokens() else {
            return None;
        };
        if token.kind != TokenType::Cmd || word.image().bytes() != self.source.as_bytes() {
            return None;
        }
        let sm = tcl_lexer::SourceMap::new(&self.source);
        let descended = descend_token(&sm, *token, word.config());
        let mut segments = segments_from_tree(descended.tree(), &sm);
        if segments.len() != 1 {
            return None;
        }
        let segment = segments.pop()?;
        let selected = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        super::diagnostic_registry::OriginalDiagnosticInvocation::new(
            selected,
            self.analysis_context(),
        )
    }

    fn original_declared_diagnostic_invocation(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
        let (segment, first) = self.original_diagnostic_segment(site)?;
        let words = crate::registry_invocation::source_structure::source_declared_command_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        (words.original_words().first()? == &first).then_some(words)
    }

    fn original_diagnostic_source(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticSource> {
        use super::diagnostic_registry::OriginalDiagnosticSource;
        self.original_diagnostic_invocation(site)
            .map(OriginalDiagnosticSource::Registry)
            .or_else(|| {
                self.original_declared_diagnostic_invocation(site)
                    .map(|words| OriginalDiagnosticSource::Declared(std::sync::Arc::new(words)))
            })
    }

    pub(super) fn original_diagnostic_source_for_segment(
        &self,
        segment: &SegmentedCommand,
    ) -> Option<super::diagnostic_registry::OriginalDiagnosticSource> {
        let command_token = *segment.argv.first()?;
        self.original_diagnostic_source(&DispatchSite {
            cmd_name: segment.texts.first()?,
            args: segment.texts.get(1..)?,
            arg_tokens: segment.argv.get(1..)?,
            arg_single: segment.single_token_word.get(1..)?,
            arg_expand_in: segment.expand_word.as_deref().unwrap_or(&[]),
            cmd_tok: command_token,
            scope_path: &[],
            presubstituted_args: self.presubstituted_args,
        })
    }

    pub(in crate::analyser) fn original_format_templates(
        original: Option<&super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) -> Vec<OriginalFormatTemplate> {
        original.map_or_else(
            Vec::new,
            super::diagnostic_registry::OriginalDiagnosticInvocation::format_templates,
        )
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
    fn dispatch_expr_arguments(&mut self, site: &DispatchSite<'_>) {
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
                &super::diagnostics::W110Anchor::ArgToken(*token),
            );
            if let Ok(content) = word.content_span() {
                self.emit_w003_dialect_invalid_expr_operator(content);
            }
            self.emit_expr_function_dialect_diagnostics(*token);
            self.emit_w114_redundant_nested_expr(text, word.span());
        }
    }

    /// Whole-tail expression assistance needs an entirely written original
    /// vector. Literal source values may be joined; dynamic words do not prove
    /// the runtime expression received by Tcl and are handled by W100 instead.
    fn dispatch_joined_source_expression(
        &mut self,
        site: &DispatchSite<'_>,
        original: &super::diagnostic_registry::OriginalDiagnosticInvocation,
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
            self.emit_w110_string_eq_ne(
                &expression,
                span,
                &super::diagnostics::W110Anchor::JoinedWords {
                    args: site.args,
                    tokens: site.arg_tokens,
                },
            );
        }
        // Standalone original operator words have authentic written anchors
        // even when another expression operand is dynamically substituted.
        self.emit_w003_dialect_invalid_expr_words(site.args, site.arg_tokens, &site.args.join(" "));
    }

    /// Generic body recursion via the command registry's
    /// `ArgRole::Body`, including `if`, `when`, `eval`, and every other
    /// registry-owned body. Sets event / conditional context and emits W105
    /// on the body argument before recursing, rather than on a nested
    /// re-segmentation.
    fn dispatch_body_arguments(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        invocation_offset: u32,
        scope_path: &[usize],
    ) {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, arg_tokens) else {
            return;
        };
        if self.vendor_source_name_policy().is_some() {
            self.dispatch_vendor_body_arguments(&tokens, args, arg_tokens, arg_single, scope_path);
            return;
        }
        let view = crate::registry_invocation::invocation_body_assistance(
            registry,
            generation.as_ref(),
            &self.command_surface(registry),
            &tokens,
        );
        let logical =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry,
                generation.as_ref().into(),
                &tokens,
                Some(self.head_identities.source_bindings_ref()),
            );
        let logical_traits = logical.as_ref().map_or(
            view.definite_traits,
            super::super::registry_invocation::LogicalStructuredInvocation::traits,
        );
        let body_indices = body_argument_indices(&view);
        if body_indices.is_empty() {
            return;
        }
        let dynamic_eval = is_dynamic_eval_body(&view);
        self.widen_irules_dynamic_eval_bindings(
            dynamic_eval,
            args,
            arg_tokens,
            &body_indices,
            scope_path,
        );
        // The `Tcl_ConcatObj` eval family: when the spec carries
        // `SCRIPT_CONCATENATES_ARGS` and words follow the first body index,
        // the script is the *concatenation* of every trailing word, not the
        // first one on its own.  Walking only the first would analyse
        // `eval set l2 hello` as the one-word script `set` — a false E002
        // plus a lost write to `l2` that then draws a false W210.
        if body_indices.first().is_some_and(|&first| {
            first + 1 < args.len()
                && logical_traits.contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        }) {
            self.dispatch_concatenated_script(
                cmd_name,
                args,
                arg_tokens,
                arg_single,
                body_indices[0],
                scope_path,
            );
            return;
        }
        let body_scope = self.selected_body_scope(registry, &view, logical.as_ref(), args);
        let (_, entered_event, prev_event) =
            self.enter_body_event_context(cmd_name, &view, args, arg_tokens, arg_single);
        let source_words =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            );
        let plan = source_words.as_ref().and_then(|words| {
            words
                .with_source_schema(generation.as_ref(), |schema| schema.clause_plan())
                .flatten()
        });
        if let (Some(plan), Some(words)) = (plan.as_ref(), source_words.as_ref()) {
            for clause in &plan.clauses {
                for index in clause.operands(ArgRole::LoopVarList) {
                    let Some(text) = words
                        .arguments()
                        .get(index)
                        .and_then(|word| word.literal_bytes())
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        continue;
                    };
                    let Some(token) = words
                        .operands()
                        .get(index)
                        .and_then(Option::as_ref)
                        .and_then(|operand| operand.word())
                        .and_then(|word| word.tokens().first())
                        .copied()
                    else {
                        continue;
                    };
                    self.define_vars_from_list(text, token, scope_path);
                }
            }
        }
        for idx in body_indices {
            if let (Some(body_text), Some(body_tok)) = (args.get(idx), arg_tokens.get(idx).copied())
            {
                self.retain_body_event_declaration(entered_event, tokens.argv.first(), body_tok);
                let effective = source_words
                    .as_ref()
                    .and_then(|words| {
                        words.origins().iter().position(|origin| {
                            *origin
                                == crate::registry_invocation::InvocationWordOrigin::Written(
                                    idx + 1,
                                )
                        })
                    })
                    .and_then(|index| index.checked_sub(1));
                let (conditional, control_flow) = body_depths(
                    effective.and_then(|_| plan.as_ref()),
                    effective.unwrap_or(idx),
                    logical_traits,
                );
                if conditional {
                    self.conditional_depth += 1;
                }
                if control_flow {
                    self.control_flow_body_depth += 1;
                }
                let is_single_token = arg_single.get(idx).copied().unwrap_or(false);
                self.dispatch_one_body_argument(
                    cmd_name,
                    body_text,
                    body_tok,
                    is_single_token,
                    scope_path,
                    body_scope,
                );
                if conditional {
                    self.conditional_depth -= 1;
                }
                if control_flow {
                    self.control_flow_body_depth -= 1;
                }
            }
        }
        if entered_event {
            self.current_event = prev_event;
        }
    }

    fn retain_body_event_declaration(
        &mut self,
        entered_event: bool,
        declaration: Option<&Span>,
        body_tok: Token,
    ) {
        if entered_event && let Some(declaration) = declaration {
            self.record_original_vendor_variable_body(
                *declaration,
                body_tok.span,
                None,
                crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Event,
            );
        }
    }

    fn selected_body_scope(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        view: &crate::registry_invocation::InvocationBodyAssistance,
        logical: Option<&crate::registry_invocation::LogicalStructuredInvocation>,
        args: &[String],
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let logical_roles = logical
            .map(super::super::registry_invocation::LogicalStructuredInvocation::written_roles);
        let logical_scope = logical
            .and_then(|selected| selected.body_scope(registry, self.analysis_context().as_ref()));
        self.record_scoped_sibling_definition(
            logical_scope.or(view.definite_scope),
            logical_roles.as_deref().unwrap_or(&view.definite_roles),
            args,
        )
    }

    /// Walk selected authored body positions independently of executable
    /// invocation admission. The retained body owns source advice only.
    fn dispatch_vendor_body_arguments(
        &mut self,
        tokens: &crate::ir::CommandTokens,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.lexer_config();
        let Some(head) = tokens.words().first().and_then(|word| {
            self.result
                .original_vendor_source_name_in_source(&image, config, word.source().span)
        }) else {
            return;
        };
        let Some(metadata) = self.original_vendor_variable_metadata(head) else {
            return;
        };
        if !metadata.roles_complete()
            || metadata.original_words().len() != args.len() + 1
            || args.len() != arg_tokens.len()
            || metadata
                .original_words()
                .iter()
                .any(|word| word.group().expand)
        {
            return;
        }
        let bodies: Option<Vec<_>> = metadata
            .roles()
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
            .map(|(argument, _)| {
                metadata
                    .argument_offset()
                    .checked_add(usize::from(*argument))
            })
            .collect();
        let Some(bodies) = bodies else {
            return;
        };
        if bodies.iter().any(|argument| *argument >= args.len()) {
            return;
        }
        let traits = metadata.possible_traits();
        let command = metadata.command();
        if bodies.first().is_some_and(|&first| first + 1 < args.len())
            && traits.contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
        {
            self.dispatch_concatenated_script(
                command, args, arg_tokens, arg_single, bodies[0], scope_path,
            );
            return;
        }
        let (_, event, previous_event) =
            self.enter_registry_event_context(command, traits, args, arg_tokens, arg_single);
        let conditional = traits.contains(tcl_registry::Traits::BRANCH_SELECTED_BODY);
        let control_flow = traits.contains(tcl_registry::Traits::CONTROL_FLOW);
        if conditional {
            self.conditional_depth += 1;
        }
        if control_flow {
            self.control_flow_body_depth += 1;
        }
        for argument in bodies {
            let body = arg_tokens[argument];
            if event && let Some(declaration) = tokens.argv.first() {
                self.record_original_vendor_variable_body(
                    *declaration,
                    body.span,
                    None,
                    crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Event,
                );
            }
            self.dispatch_one_body_argument(
                command,
                &args[argument],
                body,
                arg_single.get(argument).copied().unwrap_or(false),
                scope_path,
                None,
            );
        }
        if conditional {
            self.conditional_depth -= 1;
        }
        if control_flow {
            self.control_flow_body_depth -= 1;
        }
        if event {
            self.current_event = previous_event;
        }
    }

    fn enter_body_event_context(
        &mut self,
        cmd_name: &str,
        view: &crate::registry_invocation::InvocationBodyAssistance,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> (bool, bool, Option<String>) {
        let spec_traits = view.definite_traits;
        // iRules event handlers are a file-level declaration surface.  A
        // handler-shaped command reached while already walking any body is
        // invalid (IRULE5006) and must not manufacture or replace event
        // context.  In particular, a nested handler inside an event retains
        // the outer event's context, while one inside a proc retains `None`.
        let direct_event_layout = view.definite_invocation.as_ref().filter(|invocation| {
            invocation.arguments.len() == args.len()
                && (0..args.len())
                    .all(|index| invocation.effective.written_argument(index) == Some(index))
        });
        self.enter_registry_event_context(
            direct_event_layout.map_or(cmd_name, |invocation| {
                invocation.facts.canonical_command.as_str()
            }),
            if direct_event_layout.is_some() {
                spec_traits
            } else {
                tcl_registry::Traits::empty()
            },
            args,
            arg_tokens,
            arg_single,
        )
    }

    /// Scoped definitions belong to a selected logical body contract and an
    /// original written name operand. The scoped analysis inventory grants no
    /// actual command-table publication, entered activation or outward write.
    fn record_scoped_sibling_definition(
        &mut self,
        body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
        roles: &[(usize, ArgRole)],
        args: &[String],
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let sibling_name_idx = body_scope
            .filter(|env| env.include_sibling_definitions)
            .and_then(|_| {
                roles
                    .iter()
                    .find_map(|&(index, role)| (role == ArgRole::Name).then_some(index))
            });
        if let (Some(env), Some(ni)) = (body_scope, sibling_name_idx)
            && let Some(name) = args.get(ni)
            && !name.is_empty()
        {
            self.result
                .scoped_sibling_defs
                .entry(env.name)
                .or_default()
                .insert(name.clone());
        }
        body_scope
    }

    /// **Measurements §4c** — the iRules dynamic-code widening. The rule
    /// compiler's load-time bans are *lexical*: they scan braced script
    /// literals (so `eval {proc …}` is rejected like bare `proc` — the
    /// recursion above handles that), but script text held in a variable
    /// escapes the scan entirely. A dynamic `eval`/`uplevel` (any level
    /// spelling) whose script head the analyser cannot read therefore
    /// makes every literal-surface check abstain AND widens the realm
    /// state: the hidden script may define procs, and a runtime-defined
    /// proc is a **persistent per-TMM global** — created once (typically
    /// in `RULE_INIT`), surviving across events, connections, and separate
    /// requests on the same TMM (§4c: `persist=YES`, identical on a
    /// second request). That is exactly the "runtime set of commands is
    /// unknowable" fact the existing dynamic-provider machinery models
    /// ([`super::types::AnalysisResult::has_dynamic_providers`] — the
    /// oracle's `CommandDomainWidening::DynamicProviders`, under which
    /// every head answers `BindingKnowledge::Unknown`), so it is wired
    /// through that flag rather than new state: W123/W120-class
    /// unresolved-name conclusions abstain for the rest of the document.
    ///
    /// Scope: iRules only, and only at top-level-or-event scope — the
    /// scopes §4c measured; the load-time *lexical* rejections themselves
    /// (a literal `proc` in a `when` body, a literal call head) are NOT
    /// softened, because the rule compiler cannot see runtime definitions
    /// either. Commands in the §4b disabled split never widen: an
    /// interpreter-absent head does not run, and a compiler-refused head
    /// never loads.
    fn widen_irules_dynamic_eval_bindings(
        &mut self,
        dynamic_eval: bool,
        args: &[String],
        arg_tokens: &[Token],
        body_indices: &[usize],
        scope_path: &[usize],
    ) {
        if self.result.has_dynamic_providers || !self.profile.is_irules() {
            return;
        }
        if !dynamic_eval {
            return;
        }
        // The script *head* is hidden exactly when the first script word is
        // a substitution (`$s`, `[build]`) or interpolates one; a bare
        // literal head (`uplevel 1 helper`) is still resolved at load time.
        let hidden_head = body_indices.first().is_some_and(|&first| {
            arg_tokens.get(first).is_some_and(|tok| {
                matches!(tok.kind, TokenType::Var | TokenType::Cmd)
                    || args.get(first).is_some_and(|text| {
                        super::diagnostics::helpers::has_substitution(text.trim(), tok)
                    })
            })
        });
        if !hidden_head {
            return;
        }
        if matches!(
            self.irules_execution_context(scope_path),
            tcl_registry::events::IrulesExecutionContext::TopLevel
                | tcl_registry::events::IrulesExecutionContext::EventBody
        ) {
            self.result.has_dynamic_providers = true;
        }
    }

    fn enter_event_context(
        &mut self,
        enters_event_context: bool,
        args: &[String],
    ) -> (bool, Option<String>) {
        if !enters_event_context {
            return (false, None);
        }
        let previous = self.current_event.clone();
        self.current_event = args.first().cloned();
        (true, previous)
    }

    fn enter_registry_event_context(
        &mut self,
        command: &str,
        traits: tcl_registry::Traits,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> (bool, bool, Option<String>) {
        let is_event_handler = traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER);
        let valid = self.irules_event_body_is_valid(
            command,
            is_event_handler,
            args,
            arg_tokens,
            arg_single,
        );
        let (entered, previous) =
            self.enter_event_context(is_event_handler && self.body_depth == 0 && valid, args);
        (valid, entered, previous)
    }

    fn irules_event_body_is_valid(
        &self,
        command: &str,
        is_event_handler: bool,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        if !self.profile.is_irules() || !is_event_handler {
            return true;
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let generation = self.analysis_context();
        let registry = generation.commands();
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            arg_tokens,
            arg_single,
        );
        let declaration = closed
            .as_deref()
            .and_then(|closed| {
                tcl_registry::events::IrulesDeclarationArguments::new(
                    &words, arg_tokens, arg_single, closed,
                )
            })
            .and_then(|arguments| {
                registry.irules_top_level_declaration(
                    command,
                    arguments,
                    &tcl_registry::events::EventRegistry::build(),
                )
            });
        self.body_depth == 0
            && matches!(
                declaration,
                Some(tcl_registry::events::IrulesTopLevelDeclaration::Event { body_index, .. })
                    if arg_tokens.get(body_index).is_some_and(|token| token.kind == TokenType::Str)
            )
    }

    /// Analyse the script a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
    /// command actually evaluates: the `Tcl_ConcatObj` join of every word from
    /// `first` (its first `ArgRole::Body` index) to the end of the call.
    ///
    /// Two outcomes, both sound:
    ///
    /// - **Every trailing word is static script text** — walk the tail as
    ///   one script via [`super::utils::concat_script_window`], which
    ///   rebuilds it *in place* (content bytes at their true source offsets,
    ///   delimiters and gaps blanked to spaces) so every span the walk
    ///   records — command references, variable reads and writes,
    ///   diagnostics — lands on the exact bytes it describes, rename-safe.
    ///   `eval set l2 hello` is analysed as `set l2 hello`, so `l2` is
    ///   recorded as written and no arity error is invented.
    /// - **Any word is dynamic** (or its bytes cannot be mapped) — consume
    ///   the command without walking, the same answer
    ///   `handle_interp_eval_command` gives a multi-word `interp eval`:
    ///   substitution happens before concatenation, so the real script is
    ///   unknowable and every definedness or arity fact derived from the
    ///   written words would be a guess.
    fn dispatch_concatenated_script(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        first: usize,
        scope_path: &[usize],
    ) {
        let (Some(words), Some(tokens)) = (args.get(first..), arg_tokens.get(first..)) else {
            return;
        };
        let Some(first_tok) = tokens.first() else {
            return;
        };
        // W105 still belongs to the written first word (`eval set …` is an
        // unbraced body however the tail concatenates), so fire it before the
        // shape decision — with that word's own single-token flag, so a bare
        // `eval $cmd arg` stays exempt exactly as `eval $cmd` is.
        self.emit_w105_unbraced_body(
            cmd_name,
            &words[0],
            *first_tok,
            arg_single.get(first).copied().unwrap_or(false),
        );
        let Some((script, span)) = super::utils::concat_script_window(words, tokens, &self.source)
        else {
            // The tail cannot be walked (a dynamic word). A braced first word
            // is still a literal script prefix — concatenation appends after
            // it, so its own commands run as written — and walking it keeps
            // every scope/definition it declares visible to the editor. The
            // `eval {script} $extra` shape (and a brace-mangled document
            // whose mis-extended body word drags real code into this arm)
            // must not lose the script to the decline.
            if first_tok.kind == TokenType::Str {
                self.analyse_body(&words[0], *first_tok, scope_path);
                return;
            }
            // Declining the walk must not lose the *reference* a `$cmd` script
            // word carries — `uplevel #0 $cmd [list x y]` still dispatches
            // whatever `$cmd` holds.
            self.record_var_body_const_dispatch(*first_tok, scope_path);
            return;
        };
        // `analyse_body` walks only a `Str` (braced) body and anchors at
        // `span.start() + content_offset`; the window's text starts at its
        // span's own first byte, so the anchor carries no delimiter to skip.
        let anchor = Token::new(TokenType::Str, span);
        self.analyse_body(&script, anchor, scope_path);
    }

    /// Analyse a single body-role argument: fires `W105`, walks the script
    /// (through a scoped command environment when `body_scope` names one),
    /// and — for a bareword (unbraced) body — also dispatches it as a
    /// zero-arg command call. Split out of [`Self::dispatch_body_arguments`]
    /// purely to keep that function under the line-count lint; the two
    /// always run together, once per body-role index.
    fn dispatch_one_body_argument(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: Token,
        is_single_token: bool,
        scope_path: &[usize],
        body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    ) {
        self.emit_w105_unbraced_body(cmd_name, body_text, body_tok, is_single_token);
        // A bareword body (`if {$cond} mymod::foo`, `uplevel 1
        // mymod::qux`) is a single, statically-known zero-arg command
        // call — a legitimate alternative form real Tcl accepts
        // identically to a braced block (the exact shape
        // `emit_w105_unbraced_body` above already exempts from its
        // own warning). `analyse_body` below only ever recurses into
        // a `Str` (braced) body, so without this such a call is
        // invisible to `command_invocations` entirely: found by
        // hover/definition (which resolve independently off the
        // cursor token) but missed by references/rename — silently
        // producing an incomplete rename that breaks the program at
        // the missed call site.
        // Dispatched through the ordinary `process_command` path
        // (not a hand-rolled invocation record) so it gets full
        // treatment: arity checking, W123, nested diagnostics —
        // everything a real call site deserves.
        if body_tok.kind == TokenType::Esc
            && !body_text.trim().contains(char::is_whitespace)
            && !super::diagnostics::helpers::has_substitution(body_text.trim(), &body_tok)
        {
            self.process_command(
                &[body_text.trim().to_string()],
                &[body_tok],
                &[true],
                &[false],
                scope_path,
            );
        }
        self.record_var_body_const_dispatch(body_tok, scope_path);
        // When the body runs in a scoped command environment, record its
        // region (so the post-walk W123 pass and the LSP providers can
        // resolve the scoped heads by position) and push the environment
        // so the in-walk arity / subcommand checks resolve them too.
        if let Some(env) = body_scope {
            let start = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.result
                .scoped_command_regions
                .push(super::types::ScopedBodyRegion {
                    span: tcl_lexer::Span::new(start, body_tok.span.end()),
                    env,
                });
            self.body_scope_stack.push(env);
            self.analyse_body(body_text, body_tok, scope_path);
            self.body_scope_stack.pop();
        } else {
            self.analyse_body(body_text, body_tok, scope_path);
        }
    }

    /// Record a bare `$var` script word (`eval $cmd`, `uplevel #0 $cmd …`) as
    /// a pending const-dispatch site: the variable's value is the command
    /// prefix actually dispatched, exactly the "value is a command prefix"
    /// shape `{*}$cmd` gets via `head_expanded`.
    ///
    /// `analyse_body` only ever recurses a literal `Str` body, so without
    /// this the site is invisible to `command_invocations` — found by
    /// hover/definition (which resolve independently off the cursor token)
    /// but missed by references/rename. Recorded the same way
    /// `record_var_or_cmd_command_site` records a command's own `$cmd`-headed
    /// dispatch, for `settle_const_dispatches` to resolve in the CFG/SSA
    /// phase.
    ///
    /// Shared by the ordinary body walk and the
    /// [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`] decline path: a
    /// multi-word `uplevel #0 $cmd [list x y]` yields no analysable script,
    /// but `$cmd` is still a real dispatch whose reference must be recorded.
    ///
    /// Guarded to a "pure" reference — [`Analyser::split_braced_head`] leaving
    /// no suffix — so a composite word like `${cmd}Suffix`, a
    /// literal-concatenated value rather than `$cmd`'s own, is left alone.
    ///
    /// Comparing the first-`}` truncation against the whole raw text would
    /// also decline a *pure* reference whose name legitimately
    /// ends in `}`: at 9.x `${a{b}}` names the variable `a{b}`, and
    /// `token_text` hands that over as `a{b}` with the closer already outside
    /// the span. Asking the shared owner answers `Unterminated` there — no
    /// closer inside the text, so all of it is the name — and the dispatch is
    /// recorded instead of dropped.
    fn record_var_body_const_dispatch(&mut self, body_tok: Token, scope_path: &[usize]) {
        if body_tok.kind != TokenType::Var {
            return;
        }
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let raw = sm.token_text(body_tok);
        let (name, suffix) = self.split_braced_head(raw);
        let var_name = name.to_string();
        if !suffix.is_empty() {
            return;
        }
        let ns = self.command_resolution_namespace(scope_path);
        self.pending_const_dispatches
            .push(super::state::ConstDispatchSite {
                var_name,
                span: body_tok.span,
                ns,
                head_expanded: true,
            });
    }

    /// Retain conditional callback source metadata from the original selected
    /// command or instance-method schema and the genuine whole prefix operand.
    fn record_command_prefix_invocations(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) {
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let Some((segment, _)) =
            crate::registry_invocation::source_structure::original_segment_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            )
        else {
            return;
        };
        let source_map = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let heads =
            nested_source_prefixes(&source_map, &self.result, &segment, self.lexer_config());
        for head in heads {
            let Some(original) = head.callback else {
                continue;
            };
            let original_lookup = original.lookup().cloned();
            let original_name_input = Some(original.name_input().clone());
            let callback_arity = original.appended_arity();
            let callback_baked_args = original.baked_argument_count();
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    original_callback_signature_lookup: None,
                    original_callback_prefix: Some(std::sync::Arc::new(original)),
                    original_lookup,
                    original_name_input,
                    lookup: crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
                    name: head.name,
                    range: head.span,
                    resolved_qualified_name: None,
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    argc: None,
                    callback_arity,
                    callback_baked_args,
                    indirect: false,
                    rename_safe: false,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }

    /// Record a **command reference** — a command named as data rather than
    /// invoked with a fixed argument list at this span (an `info body` target,
    /// a `forward` / ensemble `-map` target, an expression math function).
    /// `written` is the head as it appears, `span` its token, `resolved` the
    /// qualified command it denotes; [`Self::finalise_invocation_resolutions`]
    /// recovers the namespace from the `(written, resolved)` pair and settles
    /// the candidate list, so a rename rewrites only the written token.  `argc`
    /// is the call-site argument count for arity checking, or `None` when the
    /// site merely names the command.
    pub(in crate::analyser) fn push_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.push_command_reference_with_policy(
            written,
            span,
            resolved,
            argc,
            false,
            crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
        );
    }

    /// [`Self::push_command_reference`] with an explicit existence policy:
    /// `existence_probe: true` records a reference the W123 pass must skip
    /// (the probed command legitimately may not exist).
    pub(in crate::analyser) fn push_command_reference_with_policy(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
        existence_probe: bool,
        lookup: crate::signature_scan::types::SignatureCommandLookup,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup,
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe,
                is_mathfunc_call: false,
                ensemble_dispatch: None,
            },
        );
    }

    /// [`Self::push_command_reference`] for the **subcommand word** of an
    /// `<ensemble> <sub> …` dispatch: an existence-probed reference to the
    /// mapped target that also carries the mapping's provenance
    /// ([`crate::signature_scan::types::SignatureCommandInvocation::ensemble_dispatch`],
    /// so rename can tell a `-map` key — an arbitrary name it
    /// must leave alone — from a `-subcommands` entry, which is the target's
    /// own tail and has to move with it.
    ///
    /// A dedicated method rather than another parameter on
    /// [`Self::push_command_reference_with_policy`]: every dispatch word is
    /// existence-probed for the same reason (`make` is never independently
    /// callable), so the two facts always travel
    /// together and callers cannot pair them wrongly.
    pub(in crate::analyser) fn push_ensemble_dispatch_reference(
        &mut self,
        written: String,
        span: Span,
        entry: &super::types::EnsembleSubcommandTarget,
        argc: Option<usize>,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup: crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
                name: written,
                range: span,
                resolved_qualified_name: Some(entry.target.clone()),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: true,
                is_mathfunc_call: false,
                ensemble_dispatch: Some(entry.provenance),
            },
        );
    }

    /// [`Self::push_command_reference`] for an `expr` math-function call —
    /// see
    /// [`crate::signature_scan::types::SignatureCommandInvocation::is_mathfunc_call`].
    /// A dedicated method rather than another positional bool on
    /// [`Self::push_command_reference_with_policy`]: it has exactly one
    /// caller, and a same-typed `existence_probe, is_mathfunc_call` pair
    /// invites a silently-transposed call.
    pub(in crate::analyser) fn push_mathfunc_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                original_callback_signature_lookup: None,
                original_callback_prefix: None,
                original_lookup: None,
                original_name_input: None,
                lookup: crate::signature_scan::types::SignatureCommandLookup::InvocationHead,
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolved_user_definition: false,
                resolved_definition: None,
                resolved_command_reference: None,
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: false,
                is_mathfunc_call: true,
                ensemble_dispatch: None,
            },
        );
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::CommandName`] argument as
    /// a command invocation: a bare command name held as data (`info body
    /// PROC`, `info args PROC`, `info default PROC …`) that navigation must
    /// reach, but which is *not* invoked here — so it carries no call arity.
    /// A dynamic word (`info body $p`) names no static command and is skipped.
    fn record_command_name_invocations(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
    ) {
        let roles = self.retained_argument_role_assistance(invocation_offset, arg_tokens);
        let definite = self.retained_argument_role_consensus(invocation_offset, arg_tokens);
        // Required-existence references and probe references share the
        // recording; only the existence policy carried on the record
        // differs (a probe never feeds W123).
        for (role, probe) in [
            (tcl_registry::arg_role::ArgRole::CommandName, false),
            (tcl_registry::arg_role::ArgRole::CommandNameProbe, true),
        ] {
            for idx in roles
                .iter()
                .filter_map(|(index, candidate)| (*candidate == role).then_some(*index))
            {
                let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                if name.is_empty() || crate::naming::is_dynamic_word(name) {
                    continue;
                }
                // A glob pattern probes a *set* of commands, not one exact
                // name — no single reference identity exists, so abstain.
                if probe && name.contains(['*', '?']) {
                    continue;
                }
                let resolved = self.resolve_command_qualified_name(name, scope_path);
                let unanimous = definite.contains(&(idx, role));
                let lookup = if unanimous {
                    crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                        invocation_offset,
                    }
                } else {
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        invocation_offset,
                    }
                };
                self.push_command_reference_with_policy(
                    name.clone(),
                    tok.span,
                    resolved,
                    None,
                    probe,
                    lookup,
                );
                if !unanimous && let Some(reference) = self.result.command_invocations.last_mut() {
                    reference.rename_safe = false;
                }
            }
        }
    }

    /// The per-command occurrence tables the registry's **argument roles**
    /// produce, recorded together because they are the same walk over the same
    /// role query:
    ///
    /// * [`tcl_registry::ArgRole::NamespaceName`] words → `namespace_refs`, so
    ///   a namespace name is a navigable symbol rather than an inert word;
    /// * computed [`tcl_registry::ArgRole::VarWrite`] /
    ///   [`tcl_registry::ArgRole::VarRead`] words → `dynamic_variable_names`,
    ///   with what the constant lattice proves about the value.
    fn record_arg_role_facts(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
        presubstituted: bool,
    ) {
        self.record_namespace_name_references(
            args,
            arg_tokens,
            scope_path,
            invocation_offset,
            presubstituted,
        );
    }

    fn record_original_scoped_body_advice(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let Some(head_word) = tokens.words().first() else {
            return;
        };
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.lexer_config();
        let vendor = self
            .result
            .original_vendor_source_name_in_source(&image, config, head_word.source().span)
            .cloned();
        // An owned hosted producer cannot fall through into C/Jim source advice.
        if self.vendor_source_name_policy().is_some() && vendor.is_none() {
            return;
        }
        let namespace = super::scope::scope_at(&self.result.global_scope, scope_path)
            .and_then(|scope| scope.naming_scope.as_ref());
        if vendor.is_none() {
            let metadata = self
                .original_static_source_name_at_span(head_word.source().span)
                .and_then(|head| {
                    crate::registry_invocation::original_conditional_registry_metadata(
                        &self.analysis_context(),
                        &tokens,
                        head,
                        namespace,
                    )
                });
            match self
                .result
                .original_conditional_registry_metadata
                .entry(invocation_offset)
            {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(metadata);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &metadata {
                        entry.insert(None);
                    }
                }
            }
        }
        for body in crate::registry_invocation::original_source_scoped_bodies(
            &self.analysis_context(),
            &tokens,
            self.original_static_source_name_at_span(head_word.source().span),
            namespace,
            vendor.as_ref(),
        ) {
            if !self.result.original_scoped_bodies.contains(&body) {
                self.result.original_scoped_bodies.push(body);
            }
        }
    }

    fn record_original_variable_receivers(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return;
        };
        let assistance = self
            .head_identities
            .original_declaration_assistance(&tokens, registry);
        let source_aliases =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                &self.source,
                &self.result,
                &tokens,
            )
            .and_then(|words| {
                crate::registry_invocation::OriginalSourceVariableAliasOperands::capture(
                    words,
                    &generation,
                )
            });
        if let Some(source_aliases) = source_aliases {
            for &(local, target, purpose) in source_aliases.operands() {
                let operand = (invocation_offset, local, target, purpose);
                if !self
                    .result
                    .original_variable_alias_operands
                    .contains(&operand)
                {
                    self.result.original_variable_alias_operands.push(operand);
                }
            }
            if !self
                .result
                .original_variable_alias_source_operands
                .contains(&source_aliases)
            {
                self.result
                    .original_variable_alias_source_operands
                    .push(source_aliases);
            }
        }
        let writes = if self.vendor_source_name_policy().is_some() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            tokens
                .words()
                .first()
                .and_then(|head| {
                    self.result.original_vendor_source_name_in_source(
                        &image,
                        self.lexer_config(),
                        head.source().span,
                    )
                })
                .and_then(|original| self.original_vendor_variable_metadata(original))
                .and_then(|shape| crate::registry_invocation::vendor_variable_write_advice(&shape))
        } else {
            crate::registry_invocation::original_variable_write_advice(
                registry,
                self.analysis_context().as_ref(),
                &tokens,
            )
        };
        let missing_receiver = assistance
            .as_ref()
            .is_none_or(|advice| !advice.variable_name_obligations.is_empty());
        self.record_conditional_variable_receivers(
            &tokens,
            argument_tokens,
            scope_path,
            missing_receiver,
            writes.is_none(),
        );
        for receiver in writes.unwrap_or_default() {
            if let Some(token) = argument_tokens.get(receiver.argument) {
                self.record_original_variable_write_advice(
                    token.span,
                    scope_path,
                    receiver.form,
                    None,
                );
            }
        }
        let Some(assistance) = assistance else {
            return;
        };
        self.retain_variable_declaration_receivers(
            invocation_offset,
            argument_tokens,
            scope_path,
            assistance,
        );
    }

    fn retain_variable_declaration_receivers(
        &mut self,
        invocation_offset: u32,
        argument_tokens: &[Token],
        scope_path: &[usize],
        assistance: crate::registry_invocation::OriginalDeclarationAssistance,
    ) {
        for &argument in &assistance.variable_name_obligations {
            if let Some(token) = argument_tokens.get(argument)
                && !self
                    .result
                    .original_variable_name_unknowns
                    .contains(&token.span)
            {
                self.result.original_variable_name_unknowns.push(token.span);
            }
        }
        if assistance.variable_alias_declarations != 0
            && !self
                .result
                .original_variable_alias_obligations
                .contains(&(invocation_offset, assistance.variable_alias_declarations))
        {
            self.result
                .original_variable_alias_obligations
                .push((invocation_offset, assistance.variable_alias_declarations));
        }
        for alias in assistance.variable_alias_operands {
            let Some(local) = argument_tokens.get(alias.local) else {
                continue;
            };
            let Some(target) = argument_tokens.get(alias.target) else {
                continue;
            };
            let operand = (invocation_offset, local.span, target.span, alias.purpose);
            if !self
                .result
                .original_variable_alias_operands
                .contains(&operand)
            {
                self.result.original_variable_alias_operands.push(operand);
            }
        }
        for receiver in assistance.variable_receivers {
            let Some(token) = argument_tokens.get(receiver.argument) else {
                continue;
            };
            self.record_original_variable_receiver(token.span, scope_path, receiver.form, false);
        }
    }

    fn record_conditional_variable_receivers(
        &mut self,
        tokens: &crate::ir::CommandTokens,
        argument_tokens: &[Token],
        scope_path: &[usize],
        missing_receiver: bool,
        missing_write: bool,
    ) {
        // Conditional catalogue metadata owns source cards separately from
        // executed handler facts. Alias grammar retains its own target/local
        // producer and cannot borrow an ordinary variable receiver here.
        if self.vendor_source_name_policy().is_none() && (missing_receiver || missing_write) {
            let conditional = tokens
                .words()
                .first()
                .and_then(|head| self.original_static_source_name_at_span(head.source().span))
                .and_then(|head| {
                    crate::registry_invocation::original_conditional_registry_metadata(
                        &self.analysis_context(),
                        tokens,
                        head,
                        self.original_variable_namespace_at(scope_path).as_ref(),
                    )
                });
            if let Some(metadata) = conditional
                && metadata.roles_complete()
                && !metadata.possible_traits().intersects(
                    tcl_registry::Traits::CREATES_SCOPE_ALIAS
                        | tcl_registry::Traits::DESTROYS_VARIABLE,
                )
            {
                for &(index, role) in metadata.roles() {
                    if !matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    ) {
                        continue;
                    }
                    let Some(argument) = metadata.argument_offset().checked_add(usize::from(index))
                    else {
                        continue;
                    };
                    let (Some(token), Some(form)) = (
                        argument_tokens.get(argument),
                        metadata.possible_variable_receiver_operand_form(argument),
                    ) else {
                        continue;
                    };
                    let declaration = role == tcl_registry::ArgRole::VarWrite
                        && form == tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
                    if declaration {
                        self.record_original_variable_write_advice(
                            token.span,
                            scope_path,
                            form,
                            Some(&metadata),
                        );
                    }
                    self.record_original_variable_receiver(
                        token.span,
                        scope_path,
                        form,
                        declaration,
                    );
                    if !metadata.obligations().is_empty()
                        && !self
                            .result
                            .original_variable_name_unknowns
                            .contains(&token.span)
                    {
                        self.result.original_variable_name_unknowns.push(token.span);
                    }
                }
            }
        }
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::NamespaceName`]
    /// argument as a [`NamespaceRef`](super::types::NamespaceRef): a word
    /// naming a namespace, which navigation must reach.
    ///
    /// A **relative** name roots against the call site's own
    /// command-resolution namespace, which is what Tcl does — pinned on
    /// tclsh 9.0.4 and 8.6.16, byte-identically: inside `namespace eval
    /// ::outer`, `namespace exists inner` answers `1` (it means
    /// `::outer::inner`) while the same words at global scope answer `0`.
    /// A proc body's current namespace is its *defining* namespace, which is
    /// exactly what [`Self::command_resolution_namespace`] reports.
    ///
    /// A computed word requires its genuine retained value input; unsupported
    /// substitutions supply no namespace identity. The declaring flag comes from the registry's
    /// [`tcl_registry::Traits::DECLARES_NAMESPACE`], never from the
    /// subcommand's spelling: `namespace eval` declares, `namespace inscope`
    /// — same argument layout, same analyser hook — does not.
    ///
    /// The **empty** literal is a real namespace name and is recorded like
    /// any other relative one.  It needs no special case, because it *is* the
    /// ordinary relative rule: `crate::naming::qualify` maps it to `::` at
    /// global scope and to `::outer::` inside `namespace eval ::outer`, which
    /// is exactly what tclsh 9.0.4 and 8.6.16 do (byte-identical). At global
    /// scope `namespace exists {}` is `1`, `namespace children {}` equals
    /// `namespace children ::`, `namespace inscope {} {namespace current}` is
    /// `::`, and `namespace eval {} {…}` reopens the global namespace
    /// (`namespace current` inside is `::`, and the variables it sets land in
    /// `::`). Inside `namespace eval ::outer` the very same word means a
    /// namespace that cannot exist: `namespace exists {}` is `0`, `namespace
    /// children {}` fails `namespace "" not found in "::outer"`, and
    /// `namespace eval {} {…}` fails `can't create namespace "": only global
    /// namespace can have empty name`.  Recording it as `::outer::` — a name
    /// nothing ever declares — makes navigation abstain there, which is the
    /// right answer.
    ///
    /// A **whitespace-only** word is not empty and is not special either:
    /// `namespace eval " " {…}` genuinely creates a namespace named `" "`,
    /// which both interpreters list as `{:: }` among `namespace children ::`.
    fn record_namespace_name_references(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        invocation_offset: u32,
        presubstituted: bool,
    ) {
        let Some(original) =
            self.original_name_source(invocation_offset, arg_tokens, presubstituted)
        else {
            return;
        };
        let roles = original.argument_roles();
        let declares = original
            .registry()
            .and_then(|registry| {
                registry.with_schema(|schema| {
                    schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::DECLARES_NAMESPACE)
                })
            })
            .unwrap_or(false);
        let here = self.command_resolution_namespace(scope_path);
        for argument in roles
            .iter()
            .filter_map(|(argument, role)| (*role == ArgRole::NamespaceName).then_some(*argument))
        {
            let Some(idx) = original.written_index(argument) else {
                continue;
            };
            let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                continue;
            };
            // Implementation contract: naming.source.original-point-operand-projection
            // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
            let original_input = self.declaration_name_policy().and_then(|policy| {
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    original.word(argument)?,
                    self.word_rules(),
                    policy,
                )
                .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
            });
            if original_input.is_none() {
                // Registry assistance can still identify a potentially naming
                // position when the original value producer is unavailable.
                if !self.result.namespace_name_unknowns.contains(&tok.span) {
                    self.result.namespace_name_unknowns.push(tok.span);
                }
                if crate::naming::is_dynamic_word(name) {
                    continue;
                }
            }
            self.result.namespace_refs.push(
                super::types::NamespaceRef::from_original(
                    self.declaration_namespace_scope(scope_path),
                    self.declaration_name_policy(),
                    name,
                    tok.span,
                    u16::from(tok.content_offset),
                    declares,
                    crate::naming::qualify(&here, name),
                )
                .with_original_input(original_input),
            );
        }
    }

    /// Record each computed variable-name argument as a
    /// [`DynamicVariableNameSite`](super::types::DynamicVariableNameSite):
    /// the word's span plus the name it provably evaluates to, when the
    /// constant lattice dominates the site.
    ///
    /// Which argument of which command names a variable is the registry's
    /// answer ([`tcl_registry::ArgRole::VarWrite`] /
    /// [`tcl_registry::ArgRole::VarRead`]); whether the word is computed is
    /// [`crate::dynamic_names::names_a_dynamic_variable`]'s.  No command name
    /// is matched here.
    ///
    /// A **brace-quoted** word is skipped: `set {$n} 1` names the variable
    /// literally called `$n` and substitutes nothing (tclsh 8.6.14: `set {$n}
    /// v; info exists {$n}` -> 1 while `info exists n` -> 0).  The word text
    /// alone cannot tell the two spellings apart, so the token kind decides —
    /// the same authority `dynamic_names::scan_command` uses for the compiler
    /// -side barrier.
    ///
    /// The resolution is [`Self::resolve_dynamic_word`]'s, which folds only
    /// through values that **dominate** the site: a branch-conditional
    /// binding, a parameter, or a `[…]` substitution yields `None`, which a
    /// consumer must read as "could be anything".
    fn record_dynamic_variable_name_sites(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        offset: u32,
        presubstituted: bool,
    ) {
        let Some(original) = self.original_name_source(offset, arg_tokens, presubstituted) else {
            return;
        };
        let roles = original.variable_name_arguments();
        for (role, writes) in [(ArgRole::VarWrite, true), (ArgRole::VarRead, false)] {
            for argument in roles
                .iter()
                .filter_map(|(argument, selected)| (*selected == role).then_some(*argument))
            {
                let Some(idx) = original.written_index(argument) else {
                    continue;
                };
                let (Some(word), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                if tok.kind == TokenType::Str
                    || !crate::dynamic_names::names_a_dynamic_variable(word)
                {
                    continue;
                }
                // A word carrying both roles (a read-modify-write target)
                // is one site, recorded under the write it also performs.
                if self
                    .result
                    .dynamic_variable_names
                    .iter()
                    .any(|s| s.span == tok.span)
                {
                    continue;
                }
                let is_single = arg_single.get(idx).copied().unwrap_or(false);
                let resolved = self.resolve_dynamic_word(word, Some(*tok), is_single, scope_path);
                self.result
                    .dynamic_variable_names
                    .push(super::types::DynamicVariableNameSite {
                        span: tok.span,
                        resolved,
                        writes,
                    });
            }
        }
    }

    /// `<ensemble> <subcommand> …` — when `resolved_cmd` names a known
    /// ensemble (a key in [`AnalysisResult::ensemble_subcommand_targets`])
    /// and the first actual argument is a static, non-`{*}`-expanded
    /// subcommand word present in that ensemble's map, record a second,
    /// existence-probed [`SignatureCommandInvocation`] for the subcommand
    /// word pointing at its resolved target — the same "referenceable but
    /// never independently callable" shape [`Self::record_command_name_invocations`]
    /// already uses for `CommandNameProbe`: `make` is
    /// never itself a valid command name (only the pair `widget make`
    /// dispatches), so it must never feed W123. Lets `definition`/`hover`/
    /// `references`/rename/call-hierarchy resolve `widget make` to
    /// `::widget::Make` through the same
    /// `resolved_qualified_name`-matching path every other indirection
    /// (alias, rename, iRules `call`) already uses — no separate per
    /// -provider ensemble-aware code needed there.
    ///
    /// [`AnalysisResult::ensemble_subcommand_targets`]: super::types::AnalysisResult::ensemble_subcommand_targets
    /// [`SignatureCommandInvocation`]: crate::signature_scan::types::SignatureCommandInvocation
    fn record_ensemble_subcommand_invocation(
        &mut self,
        resolved_cmd: &str,
        args: &[String],
        arg_tokens_in: &[Token],
        arg_expand_in: &[bool],
    ) {
        // `arg_expand_in[0]` is the command name's own (always-`false`)
        // flag; `arg_expand_in[1]` is the subcommand word's — a `{*}`
        // -expanded subcommand names no static word at all.
        if arg_expand_in.get(1).copied().unwrap_or(false) {
            return;
        }
        let Some(sub) = args.first() else { return };
        if crate::naming::is_dynamic_word(sub) {
            return;
        }
        let Some(tok) = arg_tokens_in.get(1) else {
            return;
        };
        // Args *after* the consumed subcommand word — not `args.len()`,
        // which would double-count the subcommand word itself against the
        // target proc's real arity.  `{*}` anywhere in the target's own
        // arguments makes the runtime count unknown, same convention as the
        // head invocation just above.
        let sub_argc = if arg_expand_in
            .get(2..)
            .is_some_and(|rest| rest.iter().any(|&e| e))
        {
            None
        } else {
            Some(args.len() - 1)
        };
        self.record_or_defer_ensemble_subcommand(resolved_cmd, sub, tok.span, sub_argc);
    }

    /// Record the existence-probed subcommand reference for
    /// `<ensemble> <sub>` — now if the ensemble's map is already known,
    /// else queued for [`Self::flush_pending_ensemble_subcommand_invocations`].
    ///
    /// The queue is filled **only** by the per-item shell pass. The
    /// whole-file DFS walks each proc/method body at its definition point,
    /// so its map already holds every ensemble a body declared earlier in
    /// the file; the shell pass defers those bodies, so an ensemble created
    /// inside `proc ::app::widget::Setup {…}` is invisible to it and every
    /// later `::app::widget show` call site was silently dropped — leaving
    /// find-references / rename / code-lens / call-hierarchy unable to
    /// enumerate call sites that go-to-definition (an on-demand lookup
    /// against the finished analysis) resolved perfectly well.
    ///
    /// The replay is gated on the ensemble's own recording preceding the
    /// call site, which is exactly the visibility the whole-file DFS has —
    /// so the two walk strategies produce identical `command_invocations`
    /// (the `per_item == analyse` corpus gate holds).
    fn record_or_defer_ensemble_subcommand(
        &mut self,
        resolved_cmd: &str,
        sub: &str,
        span: Span,
        argc: Option<usize>,
    ) {
        if let Some(entry) = self
            .result
            .resolve_ensemble_subcommand(resolved_cmd, sub)
            .cloned()
        {
            self.push_ensemble_dispatch_reference(sub.to_owned(), span, &entry, argc);
            return;
        }
        // Nothing deferred yet means nothing can *become* visible before
        // this offset either, so there is no candidate to hold.
        if !self.defer_proc_bodies || self.deferred_bodies.is_empty() {
            return;
        }
        self.pending_ensemble_subcommands
            .push(super::state::PendingEnsembleSubcommand {
                ensemble: resolved_cmd.to_owned(),
                sub: sub.to_owned(),
                span,
                argc,
            });
    }

    /// Replay every [`Self::record_or_defer_ensemble_subcommand`] miss
    /// against the finished `ensemble_subcommand_targets` map, once every
    /// deferred body has been walked and grafted.
    ///
    /// A candidate counts only when the ensemble's own
    /// `namespace ensemble create|configure` recording **precedes** the call
    /// site — the whole-file DFS's visibility rule, reproduced here so the
    /// per-item walk records the same invocation set. Where the ensemble's
    /// name or its `-map` is dynamic nothing is recorded at all (the map
    /// never gains the entry), so the abstention is inherited from the one
    /// place that decides it rather than re-decided here.
    pub(super) fn flush_pending_ensemble_subcommand_invocations(&mut self) {
        let pending = std::mem::take(&mut self.pending_ensemble_subcommands);
        for cand in pending {
            let Some(entry) = self
                .result
                .resolve_ensemble_subcommand(&cand.ensemble, &cand.sub)
                .cloned()
            else {
                continue;
            };
            let Some(&declared_at) = self.ensemble_record_offsets.get(&cand.ensemble) else {
                continue;
            };
            if declared_at >= cand.span.start() {
                continue;
            }
            self.push_ensemble_dispatch_reference(cand.sub, cand.span, &entry, cand.argc);
        }
    }

    /// Walk every argument's source slice for ``[cmd ...]``
    /// substitutions and record each nested head as its own
    /// ``CommandInvocation``.  Extracted from
    /// [`Self::process_command`] for readability — without this,
    /// calls embedded inside argument expressions
    /// (``set x [helper $foo]``, ``puts "got [count $items]"``,
    /// ``if { [HTTP::uri] eq "/foo" }``) aren't tracked, which
    /// breaks workspace usage counts, find-references, rename,
    /// and call-hierarchy.
    fn record_nested_invocations_from_args(
        &mut self,
        arg_tokens_in: &[Token],
        scope_path: &[usize],
    ) {
        // Which arguments are *expressions*?  A `[...]` inside a braced
        // expr arg is a real invocation (`if {[acl_ok]} …`), but a `[...]`
        // inside a braced *data* word is literal (`set x {[noeval]}`) — so
        // a braced word is scanned only when it is an `Expr` arg.  A braced
        // *body* arg is covered separately by `analyse_body`.
        let expr_indices: Vec<usize> = arg_tokens_in
            .first()
            .map(|head| {
                self.retained_argument_role_assistance(
                    head.span.start(),
                    arg_tokens_in.get(1..).unwrap_or(&[]),
                )
                .into_iter()
                .filter_map(|(ordinal, role)| (role == ArgRole::Expr).then_some(ordinal))
                .collect()
            })
            .unwrap_or_default();
        // A command whose *name* is itself a substitution (`[x] hi`):
        // descend a `Cmd` head too, since every token including the
        // head can hold a nested invocation (the head's *name* is
        // recorded separately by `process_command`; this records what
        // it substitutes).
        if let Some(head) = arg_tokens_in.first()
            && head.kind == TokenType::Cmd
        {
            self.record_invocations_from_cmd_token(*head, scope_path);
        }
        for (i, arg_tok) in arg_tokens_in.iter().enumerate().skip(1) {
            let arg_start = arg_tok.span.start();
            let arg_end = arg_tok.span.end() as usize;
            let src_len = self.source.len();
            if arg_start as usize >= src_len || arg_end > src_len {
                continue;
            }
            if arg_tok.kind == TokenType::Cmd {
                self.record_invocations_from_cmd_token(*arg_tok, scope_path);
            } else if arg_tok.kind == TokenType::Str {
                // Braced word: scan its `[...]` substitutions only when it
                // is an expression argument (substitutions are then active);
                // a braced data word stays opaque (a non-expr braced word
                // is never walked as a script).
                if expr_indices.contains(&(i - 1)) {
                    self.record_invocations_from_expr_token(*arg_tok, scope_path);
                }
            } else {
                // `Esc` (bareword / quoted): substitutions are active, so
                // scan.  Clone the slice into an owned ``String`` so the
                // helper can take ``&mut self`` without conflicting with the
                // source borrow.
                let Some(arg_src) =
                    Analyser::source_slice(&self.source, arg_start as usize, arg_end)
                        .map(str::to_owned)
                else {
                    continue;
                };
                self.record_invocations_from_word_token(*arg_tok, &arg_src, arg_start, scope_path);
            }
        }
    }

    /// Inner: ``Cmd`` (``[…]``) substitution tokens.  Descend the
    /// substitution into a child CST ([`descend_token`]), segment it,
    /// and record *every* inner command's bareword head, recursing
    /// into nested ``[...]``.
    ///
    /// A flat [`first_command_head`] scan would record only the
    /// *first* head of each ``[...]``, dropping ``;``- / newline-
    /// separated commands (`[foo; bar]` → only `foo`); the CST
    /// descent finds them all.
    fn record_invocations_from_cmd_token(&mut self, arg_tok: Token, scope_path: &[usize]) {
        let config = self.lexer_config();
        // Collect the inner heads first (this borrows `self.source`
        // through the `SourceMap`); resolve + push afterwards so the
        // immutable source borrow has ended.
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            // `arg_tok` is the *merged* argv token.  For a compound word
            // whose first fragment is a `[…]` substitution (`[foo]bar`,
            // `[foo]$x`, `[foo]bar[baz]`), `segments_from_tree` widens the
            // span from the first fragment's start to the *last* fragment's
            // end.  Descending that merged span would re-lex the trailing
            // literal as a script and record a bogus head (`[foo]bar` →
            // `foo]bar`).  Descend each `[…]` fragment instead, walking the
            // unmerged token stream; a single-fragment `[…]` word yields
            // just itself.
            for frag in self.cmd_fragments(arg_tok, config) {
                collect_substitution_heads(
                    &sm,
                    &self.result,
                    frag,
                    config,
                    &mut heads,
                    &mut expr_toks,
                );
            }
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // Math-function applications inside any nested `[expr {…}]` dispatch
        // to `::tcl::mathfunc::<fn>` — recorded here (with `&mut self`) because
        // the free-function collection can't resolve that namespace.
        for expr_tok in expr_toks {
            self.record_expr_function_invocations(expr_tok, scope_path);
        }
    }

    /// Recognise the tcllib `namespace eval $ns [list namespace unknown
    /// $handler]` idiom: the ``[...]`` body is a
    /// `Cmd` token, so [`Self::analyse_body`]'s literal-`{...}`-only gate
    /// never walks it as a script, and the generic nested-substitution
    /// scan resolves the segment's head to `list` (never dispatching
    /// `AnalyserHookId::NamespaceUnknown`) — so the handler installation
    /// is invisible to every existing path. Narrowly recognises the
    /// exact `list namespace unknown ?HANDLER?` shape and, on a match,
    /// calls [`Self::handle_namespace_unknown_command`] unmodified with
    /// the quoted command's `["unknown", HANDLER?]` args slice, reusing its
    /// established empty/query-form gating rather than reimplementing
    /// it. Both halves are registry facts: the `list` build is the command
    /// carrying `BUILDS_COMMAND_PREFIX`, and the quoted command is the one
    /// the `NamespaceUnknown` hook is stamped on.
    ///
    /// Deliberately narrow: does not recognise the same idiom built via
    /// `concat`, `format`, `linsert`, string concatenation, or a
    /// `list`-building helper proc — a documented scope boundary, not
    /// an oversight (no attested real-world instance of those forms).
    pub(super) fn detect_list_wrapped_namespace_unknown(&mut self, body_tok: Token) {
        if body_tok.kind != TokenType::Cmd {
            return;
        }
        let config = self.lexer_config();
        // Collect the descended segments first (this borrows
        // `self.source` through the `SourceMap`); call the `&mut self`
        // handler afterwards, once the immutable borrow has ended — the
        // same two-phase shape as `record_invocations_from_cmd_token`.
        let segs: Vec<SegmentedCommand> = {
            let sm = Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut segs = Vec::new();
            for frag in self.cmd_fragments(body_tok, config) {
                if frag.kind != TokenType::Cmd || sm.token_text(frag).is_empty() {
                    continue;
                }
                let descended = descend_token(&sm, frag, config);
                segs.extend(segments_from_tree(descended.tree(), &sm));
            }
            segs
        };
        let Some(input) = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())
            .cloned()
        else {
            return;
        };
        let context = input.context_registry();
        for seg in &segs {
            // `list HEAD word …` quotes the command it builds — the
            // registry's `BUILDS_COMMAND_PREFIX` reading — and that command
            // installs the handler when it resolves to the hook `namespace
            // unknown` is stamped with: no spelling is compared here.
            let Some(words) = crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(
                &self.source,
                &input,
                &self.head_identities,
                seg,
            ) else {
                continue;
            };
            let Some(quoted) = crate::script_arg::original_list_built_script_command(&words, seg)
            else {
                continue;
            };
            if !(2..=3).contains(&quoted.texts.len())
                || words.with_source_schema(&context, |schema| schema.semantics.analyser_hook)
                    != Some(Some(tcl_registry::hooks::AnalyserHookId::NamespaceUnknown))
            {
                continue;
            }
            self.handle_namespace_unknown_command(quoted.args());
        }
    }

    /// Run the per-command syntactic dispatch
    /// ([`Self::emit_dispatch_site_diagnostics`] — security W101-W312,
    /// bounds W230-W242, W001 / W004 / W304, arity E002-E003, …) on every
    /// command nested in a ``[…]`` substitution of this command's words,
    /// recursing into further nested substitutions and the nested
    /// commands' own bodies.  The main analyser walk descends proc /
    /// control-flow *bodies* (so those commands are already checked) but
    /// never a ``[…]`` substitution, which it treats as an opaque value —
    /// so `set fh [open "|$cmd" r]` / `set x [string index abc 99]` would
    /// otherwise escape the per-command checks.
    ///
    /// Only ``[…]`` regions are entered here; everything reached from
    /// inside one is invisible to the main walk, so the recursion may
    /// freely descend the nested commands' own bodies and substitutions
    /// without double-firing a diagnostic the main walk already emitted.
    /// `scope_path` is the enclosing command's scope — a substitution
    /// runs in the same frame as the command it is embedded in.
    fn run_nested_command_diagnostics(&mut self, arg_tokens_in: &[Token], scope_path: &[usize]) {
        // Collect the descended substitution commands first (this borrows
        // `self.source` through the `SourceMap`); run the `&mut self`
        // dispatch afterwards, once the immutable borrow has ended.  Each
        // `SegmentedCommand` is fully owned (absolute spans), so it
        // outlives the borrow.
        let config = self.lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for arg_tok in arg_tokens_in {
                let start = arg_tok.span.start() as usize;
                let end = arg_tok.span.end() as usize;
                if start > self.source.len() || end > self.source.len() || start > end {
                    continue;
                }
                match arg_tok.kind {
                    TokenType::Cmd => {
                        for frag in self.cmd_fragments(*arg_tok, config) {
                            collect_substitution_segments(
                                &sm,
                                &self.result,
                                frag,
                                config,
                                &mut nested,
                            );
                        }
                    }
                    // A quoted / bareword / compound word (`Esc`) may carry
                    // live `[...]` substitutions (`log "got [HTTP::uri]"`).
                    // The bare-`Cmd` walk never enters it, so its substitution
                    // commands escape every per-command check (IRULE3102, W123,
                    // …).  A braced `Str` data word's `[...]` is literal and is
                    // skipped; braced *expr* args are covered by
                    // `run_nested_expr_diagnostics`.
                    TokenType::Esc => {
                        let Some(arg_src) = Analyser::source_slice(&self.source, start, end) else {
                            continue;
                        };
                        if !arg_src.contains('[') {
                            continue;
                        }
                        for (off, inner) in top_level_cmd_subst_regions(arg_src) {
                            let off = u32::try_from(off)
                                .expect("byte offset fits in u32 for in-memory source");
                            let base = arg_tok.span.start() + off;
                            for seg in crate::segmenter::segment_commands_with_offset_and_config(
                                inner, base, config,
                            ) {
                                collect_segment_recursive(
                                    &sm,
                                    &self.result,
                                    seg,
                                    config,
                                    &mut nested,
                                    0,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    /// Run the per-command syntactic dispatch on every command nested in a
    /// ``[…]`` substitution that appears inside a *braced expression*
    /// argument (`if { [acl_ok] } …`, `while { [done $x] } …`).  Such an
    /// argument is a `Str` (braced) token, so it is opaque to
    /// [`Self::run_nested_command_diagnostics`] (which only descends bare
    /// `Cmd` argument tokens) — yet the expression's `[…]` substitutions
    /// are live commands the main walk never reaches.
    fn run_nested_expr_diagnostics(&mut self, site: &DispatchSite<'_>) {
        let Some(original) = self.original_diagnostic_source(site) else {
            return;
        };
        let Some(expressions) = original.expression_arguments() else {
            return;
        };
        let expr_indices = expressions
            .arguments
            .into_iter()
            .filter_map(|argument| original.written_index(argument))
            .collect::<Vec<_>>();
        let arg_tokens = site.arg_tokens;
        let scope_path = site.scope_path;
        if expr_indices.is_empty() {
            return;
        }
        let config = self.lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for idx in expr_indices {
                // Only a *braced* expr arg is opaque to the bare-`Cmd` walk;
                // an unbraced `[…]` expr arg is itself a `Cmd` token already
                // descended by `run_nested_command_diagnostics`.
                let Some(tok) = arg_tokens.get(idx) else {
                    continue;
                };
                if tok.kind != TokenType::Str {
                    continue;
                }
                // Re-lex the braced expression as a script: its operands
                // (`$x`, `+`, literals) are not commands, but each `[…]`
                // substitution still tokenises as a `Cmd` to descend.
                let descended = descend_token(&sm, *tok, config);
                for seg in segments_from_tree(descended.tree(), &sm) {
                    for inner in &seg.all_tokens {
                        if inner.kind == TokenType::Cmd {
                            collect_substitution_segments(
                                &sm,
                                &self.result,
                                *inner,
                                config,
                                &mut nested,
                            );
                        }
                    }
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    /// Run the full per-command diagnostic dispatch on one command
    /// descended from a ``[…]`` substitution: the syntactic emitters
    /// ([`Self::emit_dispatch_site_diagnostics`]) plus the EXPR-argument
    /// walk ([`Self::dispatch_expr_arguments`], which hosts W100 / W110 /
    /// W114 / W003).  The main walk runs both on a top-level command but
    /// only the syntactic half reaches substitution commands by
    /// default, so an unbraced `expr` inside `[…]`
    /// (`set y [expr $a + $b]`) would otherwise escape W100.
    fn dispatch_nested_segment(&mut self, seg: &SegmentedCommand, scope_path: &[usize]) {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            return;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_tok = seg.argv[0];
        self.observe_interp_visibility(&seg.argv, scope_path);
        let args = seg.texts.get(1..).unwrap_or(&[]);
        let arg_tokens = seg.argv.get(1..).unwrap_or(&[]);
        let arg_single = seg.single_token_word.get(1..).unwrap_or(&[]);
        self.record_original_variable_receivers(cmd_tok.span.start(), arg_tokens, scope_path);
        self.record_dynamic_variable_name_sites(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            cmd_tok.span.start(),
            false,
        );
        // `emit_arity_diagnostics` expects the expand array parallel to
        // the *full* argv (head at index 0), matching `process_command`.
        let arg_expand = seg.expand_word.as_deref().unwrap_or(&[]);
        let site = DispatchSite {
            cmd_name: &cmd_name,
            args,
            arg_tokens,
            arg_single,
            arg_expand_in: arg_expand,
            cmd_tok,
            scope_path,
            // A command written inside a `[…]` substitution is written
            // source, with its own words: `$x` here really is a substitution
            // the author typed. (The list-built path never reaches this
            // walker — `process_command` skips the nested-substitution scan
            // entirely when its words are pre-substituted.)
            presubstituted_args: false,
        };
        self.emit_dispatch_site_diagnostics(&site);
        self.dispatch_expr_arguments(&site);
        // W216 (broken brace-form array access, `${arr}(idx)` / `${arr($i)}`)
        // must reach substitution commands too: `set v [puts ${arr}(name)]`
        // hides the offending word inside a `[…]`, which the main `walk_body`
        // pass treats as an opaque value.  Without this the nested word escapes
        // the check entirely, so the brace-then-paren emitter must run on
        // substitution commands too.
        self.emit_w216_brace_then_paren(seg);
        // A `VarWrite`-role command nested in a `[…]` substitution still
        // writes its variable arguments into the enclosing scope — the
        // idiomatic `if {[regexp {…} $s m]} {…}` / `set n [scan $s "%d" x]` /
        // `while {[gets $chan line] >= 0} {…}` — so bind them for
        // completion/hover/definition and read-before-set, just as the
        // top-level `process_command` path does.
        self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
        // Record the nested command's variable-/command-substitution-as-command
        // call site too (`puts [$obj method]`, `if {[$obj ok]} …`).  The main
        // walk treats `[…]` as a value, so without this the W307 multi-dispatch
        // suppression under-counts `$obj` dispatches that live inside command
        // substitutions and the W307/W308 emitters never see them.
        self.record_var_or_cmd_command_site(VarOrCmdSite {
            cmd_name: &cmd_name,
            cmd_tok,
            head_expanded: arg_expand.first().copied().unwrap_or(false),
            args,
            arg_tokens,
            arg_expand: arg_expand.get(1..).unwrap_or(&[]),
            scope_path,
        });
        // W125 (orphaned keyword) and IRULE5005 (direct iRules-proc call
        // without `call`) key off whether the head resolves to a user proc, so
        // they must reach substitution commands too: `when HTTP_REQUEST { set x
        // [helper] }` invokes `helper` directly inside a `[…]`, and without this
        // the IRULE5005 emitter never sees it.  Matches the top-level
        // `process_command` ordering (proc-resolution after site recording).
        self.emit_proc_resolution_diagnostics(&cmd_name, args, cmd_tok, scope_path);
        // A namespace-name argument nested in a `[…]` substitution names the
        // same namespace a top-level one would — and this is the *dominant*
        // real shape: `set targets [namespace children ::tomato]` is the very
        // line this was mined from.  The main walk treats `[…]` as an
        // opaque value, so without this the occurrence would be invisible to
        // go-to-definition / hover / find-references exactly where it matters
        // most.
        self.record_namespace_name_references(
            args,
            arg_tokens,
            scope_path,
            cmd_tok.span.start(),
            false,
        );
        // A `package require` nested in a `[…]` substitution still runs — the
        // guarded-optional-dependency idiom puts it exactly there
        // (`if {[catch {package require Tk} err]} { … fallback … }`), and the
        // body collector descends `catch`'s script argument to reach it. Without
        // this, W120 ("requires `package require Tk`") false-positives on every
        // file using the standard guard, and the W123 conservative
        // any-require-seen gate never engages.  Only the two `package`
        // hooks run here — the substitution path deliberately dispatches
        // no other handler family.
        match self.resolve_analyser_hook(&cmd_name, args) {
            Some(tcl_registry::hooks::AnalyserHookId::PackageRequire) => {
                self.handle_package_require(&cmd_name, cmd_tok, args, arg_tokens);
            }
            Some(tcl_registry::hooks::AnalyserHookId::PackageProvide) => {
                self.handle_package_provide(cmd_tok, args, arg_tokens);
            }
            Some(tcl_registry::hooks::AnalyserHookId::Catch) => {
                self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
            }
            _ => {}
        }
        // A definition command (`proc`, a class definer, `oo::define`) or an
        // `apply` lambda nested inside a substitution — the feature-detection
        // idiom `if {![catch {oo::configurable create Greeter {…}}]} {…}`, or
        // the ordinary `set r [apply {{…} {…}} …]` — still defines its
        // procedure/class and walks its body **in the body's own scope**,
        // exactly as the top-level dispatch does.  Without this the definer's
        // member keywords (`property`, `constructor`) and the defined name
        // (`Greeter`) all draw W123 as unknown commands, even though W002
        // already reported the dialect-gated definer once, and a lambda body
        // reached this way is walked by nothing at all.  The generic
        // collector descends none of these
        // bodies — a definer's by `nested_source_bodies`, a lambda's
        // because `apply`'s script argument is `ArgRole::LambdaLiteral`, which
        // the shared script-body owner deliberately does not resolve — so no body is ever
        // also dispatched as a plain script in the *enclosing* scope.  The
        // dispatch mirrors the top-level chain exactly: the stamped `Proc` /
        // `OoDefine` / `Apply` hooks first (the handlers do not name-guard
        // themselves), then the grammar-driven definer trio only on the
        // hookless path.
        // Each handler returns whether it claimed the command; nothing
        // follows this dispatch, so an unclaimed command simply ends the
        // substitution walk the same way a claimed one does.
        if self
            .dispatch_original_class_configuration(cmd_tok, scope_path)
            .is_some()
        {
            return;
        }
        {
            use tcl_registry::hooks::AnalyserHookId as Hook;
            match self.resolve_analyser_hook(&cmd_name, args) {
                Some(Hook::Proc) => {
                    self.handle_proc_command(&cmd_name, args, arg_tokens, arg_single, scope_path);
                }
                Some(Hook::OptProc) => {
                    self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path);
                }
                Some(Hook::OoDefine) => {
                    self.handle_oo_define_command(
                        &cmd_name, args, arg_tokens, arg_single, scope_path,
                    );
                }
                // `apply {{params} body ?ns?}` — the handler builds the
                // lambda's own `Proc` scope rooted at the lambda's namespace
                // (element 2, or `::`), binds its parameters there, and walks
                // the body in it.  Routing the substitution-position call
                // through the same handler is what keeps the lambda's frame
                // isolated: a variable the body sets is a local of the lambda,
                // not of the enclosing proc, and a bareword call inside it
                // resolves in the lambda's namespace.
                Some(Hook::Apply) => {
                    self.handle_apply_command(args, arg_tokens, scope_path);
                }
                None => {
                    self.note_pack_definer(&cmd_name);
                    let _claimed = self
                        .handle_oo_class_command(&cmd_name, args, arg_tokens, scope_path, cmd_tok)
                        || self.handle_snit_type_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_itcl_class_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_jim_class_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_jim_class_member_call(
                            &cmd_name, args, arg_tokens, scope_path, cmd_tok,
                        );
                }
                Some(_) => {}
            }
        }
    }

    /// The `[…]` substitution fragment tokens of a (possibly compound)
    /// `Cmd`-headed word, with absolute spans.  Re-lexing the word slice
    /// recovers the per-fragment boundaries the argv merge erased, so
    /// `[foo]bar` yields its `[foo]` fragment (not the whole word) and
    /// `[foo]bar[baz]` yields both `[foo]` and `[baz]`.  On a lex error or
    /// a degenerate empty/out-of-bounds span, falls back to the token as
    /// given so the caller still descends *something*.
    fn cmd_fragments(&self, arg_tok: Token, config: LexerConfig) -> Vec<Token> {
        let start = arg_tok.span.start() as usize;
        let end = arg_tok.span.end() as usize;
        if start >= end {
            return vec![arg_tok];
        }
        let Some(word_src) = Analyser::source_slice(&self.source, start, end) else {
            return vec![arg_tok];
        };
        let base = arg_tok.span.start();
        let frags: Vec<Token> = Lexer::with_source_map(SourceMap::new(word_src), config)
            .tokenise_all()
            .map(|toks| {
                toks.into_iter()
                    .filter(|t| t.kind == TokenType::Cmd)
                    .map(|t| Token {
                        kind: t.kind,
                        span: Span::new(t.span.start() + base, t.span.end() + base),
                        content_offset: t.content_offset,
                        in_quote: t.in_quote,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if frags.is_empty() {
            vec![arg_tok]
        } else {
            frags
        }
    }

    /// Inner: a braced ``Str`` *expression* argument (`if {[acl_ok]} …`).
    /// Record the command substitutions inside the expression — the
    /// expression's own operands are not commands (see
    /// [`collect_expr_substitutions`]).  Skips the over-recording the
    /// generic word scanner would do on a braced *data* word.
    fn record_invocations_from_expr_token(&mut self, expr_tok: Token, scope_path: &[usize]) {
        let config = self.lexer_config();
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            collect_expr_substitutions(
                &sm,
                &self.result,
                expr_tok,
                config,
                &mut heads,
                &mut expr_toks,
            );
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // This expr's own math functions, plus any nested inside a `[expr {…}]`
        // substitution the collection surfaced (`if {[expr {Pi()}]}`).
        self.record_expr_function_invocations(expr_tok, scope_path);
        for nested in expr_toks {
            self.record_expr_function_invocations(nested, scope_path);
        }
    }

    /// Every math-function application inside the expression `expr_tok`, as
    /// `(name, name_span, arg_count)` with the function-name span mapped to
    /// absolute source coordinates.  The single extraction the invocation
    /// recorder and the dialect-availability diagnostic both read.
    ///
    /// The expression body starts past the opening delimiter; the AST's
    /// offsets are relative to the *trimmed* text, so each is translated back
    /// through the leading-whitespace trim to a source span.
    pub(in crate::analyser) fn expr_function_calls(
        &self,
        expr_tok: Token,
    ) -> Vec<(String, Span, usize)> {
        if !self.result.allows_lexical_declaration_advice() {
            let generation = self.analysis_context();
            let registry = generation.commands();
            let image = tcl_lexer::SourceImage::document(&self.source);
            return self
                .head_identities
                .source_bindings_ref()
                .original_math_functions_in_source(
                    registry,
                    &image,
                    self.lexer_config(),
                    expr_tok.span,
                )
                .into_iter()
                .map(|call| {
                    (
                        call.function().to_owned(),
                        call.span(),
                        call.argument_count(),
                    )
                })
                .collect();
        }
        let content_start = expr_tok.span.start() + u32::from(expr_tok.content_offset);
        let (start, end) = (content_start as usize, expr_tok.span.end() as usize);
        let Some(expr_text) = Analyser::source_slice(&self.source, start, end) else {
            return Vec::new();
        };
        let trimmed = expr_text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let trim_base = u32::try_from(expr_text.len() - expr_text.trim_start().len()).unwrap_or(0);
        let Some(context) = self.original_expression_parser_context() else {
            return Vec::new();
        };
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(trimmed, &context);
        parsed
            .function_calls()
            .into_iter()
            .map(|(name, rel_start, argc)| {
                let name_start = content_start + trim_base + rel_start;
                let span = Span::new(
                    name_start,
                    name_start + u32::try_from(name.len()).unwrap_or(0),
                );
                (name.to_owned(), span, argc)
            })
            .collect()
    }

    /// Record each math-function application (`sin($x)`, `max($a, $b)`) as an
    /// invocation of the command it dispatches to, `::tcl::mathfunc::<name>`.
    /// A user who defines `proc ::tcl::mathfunc::myfunc { … }` — or, per TIP
    /// 232, a namespace-local `proc ::ns::tcl::mathfunc::myfunc` that shadows
    /// it inside `::ns` — then gets go-to-definition, references, rename, and
    /// arity checking on `myfunc(...)` calls, and the function is no longer
    /// reported unused.
    ///
    /// The written head is the bare function word (`sin`); the resolved name
    /// is the *local-first* candidate `{ns}::tcl::mathfunc::sin` for the
    /// call's own namespace, computed the same way an ordinary bareword
    /// command's walk-time guess is ([`Self::resolve_command_qualified_name`])
    /// so a rename rewrites only the tail token in the expression.
    /// [`Self::finalise_invocation_resolutions`] settles it against the real
    /// two-candidate rule (the caller's `tcl::mathfunc`, else the global
    /// one) — never the generic one-hop `{ns}::{name}` suffix-strip it uses
    /// for an ordinary call, which would misparse the fixed `tcl::mathfunc`
    /// dispatch segment as if it were the calling namespace and could
    /// mis-resolve to an unrelated global command sharing the bare tail name.
    fn record_expr_function_invocations(&mut self, expr_tok: Token, scope_path: &[usize]) {
        for (name, span, argc) in self.expr_function_calls(expr_tok) {
            let resolved =
                self.resolve_command_qualified_name(&format!("tcl::mathfunc::{name}"), scope_path);
            self.push_mathfunc_command_reference(name, span, resolved, Some(argc));
        }
    }

    /// Resolve each collected `(name, span, argc)` head to a qualified name and
    /// push it as a `command_invocations` entry.  `argc` carries the nested call's
    /// statically-known argument count (`None` when `{*}`-expanded), so a wrong-arg
    /// call to a cross-file proc *inside a substitution* (`set x [helper a b c]`)
    /// still draws the cross-file arity error.
    fn push_collected_heads(&mut self, heads: Vec<CollectedHead>, scope_path: &[usize]) {
        for CollectedHead {
            name,
            span: range,
            argc,
            callback,
            ensemble: sub_candidate,
        } in heads
        {
            let original_lookup = callback
                .as_ref()
                .and_then(|prefix| prefix.lookup().cloned());
            let original_name_input = callback.as_ref().map(|prefix| prefix.name_input().clone());
            let callback_arity = callback
                .as_ref()
                .and_then(crate::command_binding::OriginalCallbackPrefix::appended_arity);
            let callback_baked_args = callback.as_ref().map_or(
                0,
                crate::command_binding::OriginalCallbackPrefix::baked_argument_count,
            );
            let lookup = if callback.is_some() {
                crate::signature_scan::types::SignatureCommandLookup::DeferredReference
            } else {
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead
            };
            let rename_safe = callback.is_none() || original_lookup.is_some();
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            // `[<ensemble> <subcommand> …]` nested inside a substitution —
            // the same existence-probed subcommand reference a top-level
            // `<ensemble> <subcommand> …` call already gets from
            // `record_ensemble_subcommand_invocation`.
            // `argc` here is "args after the head" (the subcommand word
            // included), so it shifts by one to become "args after the
            // subcommand word" — the same convention that function uses.
            if let Some((sub, sub_span)) = sub_candidate {
                let sub_argc = argc.map(|a| a.saturating_sub(1));
                self.record_or_defer_ensemble_subcommand(&resolved, &sub, sub_span, sub_argc);
            }
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    original_callback_signature_lookup: None,
                    original_callback_prefix: callback.clone().map(std::sync::Arc::new),
                    original_lookup,
                    original_name_input,
                    lookup,
                    name,
                    range,
                    resolved_qualified_name: Some(resolved),
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    argc,
                    callback_arity,
                    callback_baked_args,
                    indirect: false,
                    rename_safe,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }

    /// Inner: ``Esc`` (bareword / quoted) and ``Str`` (braced)
    /// tokens.  For ``Str``, strip the surrounding ``{…}`` via
    /// ``content_offset`` first — otherwise the scanner sees the
    /// outer ``{`` and skips the entire braced region opaquely,
    /// missing every nested ``[cmd]`` inside braced expr args.
    fn record_invocations_from_word_token(
        &mut self,
        arg_tok: Token,
        arg_src: &str,
        arg_start: u32,
        scope_path: &[usize],
    ) {
        let (inner_src, inner_base) = if matches!(arg_tok.kind, TokenType::Str) {
            let inner_off = arg_tok.content_offset as usize;
            if inner_off <= arg_src.len() {
                let trimmed = arg_src[inner_off..]
                    .strip_suffix('}')
                    .unwrap_or(&arg_src[inner_off..]);
                let inner_off_u32 = u32::try_from(inner_off)
                    .expect("content_offset fits in u32 for in-memory source");
                (trimmed, arg_start + inner_off_u32)
            } else {
                (arg_src, arg_start)
            }
        } else {
            (arg_src, arg_start)
        };
        for (name, off) in scan_nested_command_heads(inner_src) {
            let abs_start = inner_base + off;
            let abs_end = abs_start
                + u32::try_from(name.len()).expect("token length fits in u32 for in-memory source");
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    lookup: crate::signature_scan::types::SignatureCommandLookup::InvocationHead,
                    original_callback_signature_lookup: None,
                    original_callback_prefix: None,
                    original_lookup: None,
                    original_name_input: None,
                    name,
                    range: tcl_lexer::Span::new(abs_start, abs_end),
                    resolved_qualified_name: Some(resolved),
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    // Nested `[cmd ...]` head, no recorded argument list — arity skip.
                    argc: None,
                    callback_arity: None,
                    callback_baked_args: 0,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }

    /// Record this command as a variable-as-command (``$obj
    /// method ...``) or command-substitution-as-command
    /// (``[expr ...] args``) call site so the post-walk W307 /
    /// W308 emitters can resolve them.
    ///
    /// # Dispatch-site shapes
    ///
    /// Four command-head shapes name an object to dispatch on, and every
    /// one of them is recognised **structurally or from the registry** —
    /// never by spelling a command name here:
    ///
    /// | Written head | Token kind | Recorded as |
    /// |---|---|---|
    /// | `$obj method` | [`TokenType::Var`] | [`DispatchReceiver::Variable`] |
    /// | `[Dog new] method` | [`TokenType::Cmd`] | a [`CmdCommandSite`] |
    /// | `objcmd method` (from `CLASS create objcmd`) | [`TokenType::Esc`] | [`DispatchReceiver::InstanceCommand`] |
    /// | `my method` | [`TokenType::Esc`] | [`DispatchReceiver::SelfDispatch`] |
    ///
    /// The last two share the bareword arm and are told apart by
    /// [`Self::record_bareword_dispatch_site`]: a registry-declared
    /// self-dispatch keyword first, the `CLASS create NAME` binding
    /// otherwise.
    ///
    /// # Span conventions
    ///
    /// Both spans a site carries are the *whole written word*, so a
    /// diagnostic anchored on either underlines exactly what the user
    /// typed. That takes deliberate work in both directions, because a
    /// token's raw span is neither:
    ///
    /// * `method_span` **trims the opening delimiter** — `content_offset`
    ///   bytes of `{` / `"` / `[` — so `{badmethod}` anchors on
    ///   `badmethod`.
    /// * `cmd_span` **adds the closing delimiter back**, via
    ///   [`tcl_lexer::word_span`], because a `Cmd` / `Str` token's span
    ///   stops at the end of its content: the raw span of `[Dog new]` is
    ///   `[Dog new`.
    ///
    /// [`CmdCommandSite`]: super::state::CmdCommandSite
    /// [`DispatchReceiver`]: super::state::DispatchReceiver
    fn record_var_or_cmd_command_site(&mut self, site: VarOrCmdSite<'_>) {
        let VarOrCmdSite {
            cmd_name,
            cmd_tok,
            head_expanded,
            args,
            arg_tokens,
            arg_expand,
            scope_path,
        } = site;
        let in_method = self.scope_path_in_method_body(scope_path);
        // Content span of the method word (delimiters trimmed) — the tight
        // W308 anchor and "did you mean" fix target.
        let method_span = arg_tokens.first().map(|t| {
            tcl_lexer::Span::new(t.span.start() + u32::from(t.content_offset), t.span.end())
        });
        match cmd_tok.kind {
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                // A composite head whose first token is a *braced* variable
                // (`${ns}::define::[…]`) merges into one Var word token, so the
                // raw text spans the whole word.  The dispatched variable is
                // only the braced name (`${ns}` → `ns`); the closer ends it and
                // the rest is a literal / substituted suffix.  Where that
                // closer sits is the release's `Tcl_ParseVarName` rule, so
                // [`Analyser::split_braced_head`] asks the shared owner rather
                // than truncating at the first `}` (a simple `$obj` or
                // namespaced `$ns::v` head has no closer at all and is
                // unchanged).
                let raw = sm.token_text(cmd_tok);
                let var_name = self.split_braced_head(raw).0.to_string();
                let method_name = args.first().cloned();
                // A simple-`$cmd` head may be a statically-known
                // dispatch.  Record the *site* for settlement in the
                // CFG/SSA phase, where the flow-sensitive value model can
                // prove (or soundly refuse to prove) the finite set of
                // command names reaching this exact program point —
                // never the walk's lexical constant map, whose
                // last-write-wins view collapses `if`/loop joins.  A
                // braced composite head (`${ns}::tail …`) is the W307
                // ensemble shape, not a whole-command variable, so it is
                // skipped.
                if var_name == raw {
                    let ns = self.command_resolution_namespace(scope_path);
                    self.pending_const_dispatches
                        .push(super::state::ConstDispatchSite {
                            var_name: var_name.clone(),
                            span: cmd_tok.span,
                            ns,
                            head_expanded,
                        });
                }
                self.var_command_sites.push(super::state::VarCommandSite {
                    var_name,
                    method_name,
                    method_span,
                    // Whole written word, so a braced `${obj}` head anchors
                    // on `${obj}` rather than `${obj` — see this function's
                    // "Span conventions".
                    cmd_span: tcl_lexer::word_span(&sm, cmd_tok),
                    in_method,
                    argc: args.len(),
                    has_expand: arg_expand.iter().any(|&e| e),
                    receiver: super::state::DispatchReceiver::Variable,
                });
            }
            TokenType::Cmd => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let cmd_text = sm.token_text(cmd_tok).to_string();
                let base = cmd_tok.span.start() + u32::from(cmd_tok.content_offset);
                let position = sm.position_at(base);
                let config = self.lexer_config();
                let map = tcl_lexer::SourceMap::new(&cmd_text).with_base(
                    base,
                    position.line,
                    position.character.get(),
                );
                let bindings = self.head_identities.source_bindings();
                let commands = crate::segmenter::segment_commands_with_offset_and_config(
                    &cmd_text, base, config,
                )
                .iter()
                .map(|command| {
                    let mut tokens =
                        crate::ir::CommandTokens::from_segmented(&map, config, command);
                    bindings.stamp_original_tokens(&mut tokens);
                    tokens
                })
                .collect();
                let method_name = args.first().cloned();
                self.cmd_command_sites.push(super::state::CmdCommandSite {
                    cmd_text,
                    commands,
                    method_name,
                    method_span,
                    cmd_span: tcl_lexer::word_span(&sm, cmd_tok),
                    in_method,
                });
            }
            TokenType::Esc => self.record_bareword_dispatch_site(BarewordDispatch {
                cmd_name,
                cmd_tok,
                args,
                method_span,
                in_method,
                arg_expand,
            }),
            _ => {}
        }
    }

    /// Capture readonly bareword dispatch sites with their whole head geometry.
    /// Registry self-dispatch vocabulary supplies a possible source shape;
    /// diagnostics retain their independent positioned member/receiver owner.
    /// Named receivers require the original source-instance issuer rather than
    /// created-command or class-report maps. Deferred scans keep genuine sites
    /// until the canonical original declarations are available after grafting.
    ///
    /// `cmd_name` is the already-extracted reporting head. It never replaces
    /// the original command token or supplies a Native receiver allocation,
    /// current method table, successful dispatch or entered frame.
    fn record_bareword_dispatch_site(&mut self, d: BarewordDispatch<'_>) {
        let BarewordDispatch {
            cmd_name,
            cmd_tok,
            args,
            method_span,
            in_method,
            arg_expand,
        } = d;
        let site = |receiver| super::state::VarCommandSite {
            var_name: cmd_name.to_string(),
            method_name: args.first().cloned(),
            method_span,
            // A bareword head has no closing delimiter, so its token span
            // already is the whole word.
            cmd_span: cmd_tok.span,
            in_method,
            argc: args.len(),
            has_expand: arg_expand.iter().any(|&e| e),
            receiver,
        };
        if self.head_is_self_dispatch_keyword(cmd_name) {
            // Recorded whether or not a class encloses this offset: the walk
            // does not yet know (an isolated per-item body has no classes at
            // all), and abstaining is the diagnosis-time job of
            // `enclosing_class_at_offset` returning `None`.
            self.var_command_sites
                .push(site(super::state::DispatchReceiver::SelfDispatch));
            return;
        }
        self.record_bareword_instance_dispatch_site(site);
    }

    /// Whether `head`, written bare in command position, is a
    /// registry-declared `TclOO` self-dispatch keyword whose next word
    /// therefore names a method on the enclosing object.
    ///
    /// Registry-first and dialect-aware: `method_dispatch_keyword` answers
    /// at the active profile's point, so a `tcl8.4` /
    /// `tcl8.5` document — with no `TclOO` at all — answers `false` for
    /// every spelling, and it resolves the `::`-qualified form itself.
    ///
    /// Purely a *shape* question, deliberately: whether this document goes
    /// on to rename, alias, delete or shadow the keyword is order-dependent
    /// and cannot be settled mid-walk, so it is asked once, post-walk, by
    /// `Analyser::self_dispatch_keyword_disturbed`.
    fn head_is_self_dispatch_keyword(&self, head: &str) -> bool {
        let generation = self.analysis_context();
        generation
            .context()
            .resolve_spec(generation.commands(), head)
            .is_some_and(|spec| {
                spec.traits
                    .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
            })
    }

    /// Named receiver inventory comes from the original source-instance issuer.
    /// Deferred scans retain complete source sites until canonical declarations
    /// join; reporting labels cannot admit a receiver or promise its lifetime.
    fn record_bareword_instance_dispatch_site(
        &mut self,
        site: impl Fn(super::state::DispatchReceiver) -> super::state::VarCommandSite,
    ) {
        let site = site(super::state::DispatchReceiver::InstanceCommand);
        if let Some(pending) = self.pending_bareword_dispatch_sites.as_mut() {
            pending.push(site);
        } else if crate::registry_invocation::source_structure::source_class_instance_words_at(
            &self.source,
            &self.result,
            site.cmd_span.start(),
        )
        .is_some()
        {
            self.var_command_sites.push(site);
        }
    }

    /// Retain class presentation from genuine original construction sites.
    /// Setter and direct named calls use their shared source receipts, which
    /// identify canonical declarations and selected family layouts. These
    /// labels are conditional source metadata, without allocation or lifetime.
    /// Per-item replay captures only original producer sites and later joins
    /// them to the grafted canonical declarations at the same source offsets.
    pub(crate) fn record_instance_creation(
        &mut self,
        cmd_name: &str,
        args: &[String],
        creation_ns: &str,
        site_offset: u32,
    ) {
        // Source class labels retain their genuine naming/setter declarations.
        // Successful publications and lifetime remain with the shared bindings.
        let bound_registry_factory = self.record_registry_factory_instance(site_offset);

        // Per-item bodies retain the exact source producer site until their
        // canonical declarations join the shell metadata during replay.
        if self.pending_instances.is_some() {
            let shape_a =
                crate::registry_invocation::source_structure::source_handle_construction_at(
                    &self.source,
                    &self.result,
                    site_offset,
                )
                .is_some();
            let shape_b = crate::registry_invocation::source_structure::source_constructor_call_at(
                &self.source,
                &self.result,
                site_offset,
            )
            .is_some();
            // A registry factory already bound above needs no user-class replay.
            if (shape_a || shape_b)
                && !bound_registry_factory
                && let Some(pending) = self.pending_instances.as_mut()
            {
                pending.push((
                    cmd_name.to_owned(),
                    args.to_vec(),
                    creation_ns.to_owned(),
                    site_offset,
                ));
            }
            return;
        }
        if bound_registry_factory {
            return;
        }
        // Genuine user-class setter syntax joins its source constructor.
        if let Some((variable, class_q)) = self.class_from_constructor_subst(site_offset) {
            self.result.instance_classes.insert(variable, class_q);
            return;
        }
        // Direct named construction retains its genuine original factory,
        // family layout and canonical class declaration. This is source class
        // presentation only; successful publication belongs to binding state.
        if let Some(call) = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            site_offset,
        ) && let Some(shape) = call.constructor_shape(&self.result)
        {
            let arguments = call.arguments();
            let name_at = match shape {
                crate::command_binding::OriginalSourceConstructorShape::Method(method) => {
                    method.names_instance_at.map(usize::from)
                }
                crate::command_binding::OriginalSourceConstructorShape::BareWord { .. } => Some(0),
            };
            if let Some(name_at) = name_at
                && let Some(name) = arguments
                    .get(name_at)
                    .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                && !name.is_empty()
                && let Some(class) = call.class_declaration().source_class(&self.result)
            {
                let class_q = class.metadata().qualified_name.clone();
                self.result
                    .instance_classes
                    .insert(name.to_owned(), class_q.clone());
                let binding = super::types::InstanceCommandBinding {
                    qualified_name: crate::naming::qualify(creation_ns, name),
                    class_q,
                };
                if !self.result.instance_command_bindings.contains(&binding) {
                    self.result.instance_command_bindings.push(binding);
                }
            }
        }
    }

    /// Retain source class labels from genuine named-factory/setter receipts.
    /// This compatibility presentation supplies no successful creation or life.
    fn record_registry_factory_instance(&mut self, site_offset: u32) -> bool {
        use tcl_registry::AuthoredSourceCommandPublicationKind::Instance;
        let named = crate::registry_invocation::source_structure::source_command_publication_at(
            &self.source,
            &self.result,
            site_offset,
        )
        .and_then(|publication| {
            let Instance { class_name } = publication.kind() else {
                return None;
            };
            let name = std::str::from_utf8(publication.name_bytes()).ok()?;
            Some((name.to_owned(), class_name.to_owned()))
        });
        if let Some((name, class)) = named {
            self.bind_registry_instance_class(name, class);
            return true;
        }
        let captured = crate::registry_invocation::source_structure::source_handle_class_advice_at(
            &self.source,
            &self.result,
            site_offset,
        )
        .and_then(|binding| {
            let Instance { class_name } = binding.factory().kind() else {
                return None;
            };
            Some((
                std::str::from_utf8(binding.variable_bytes())
                    .ok()?
                    .to_owned(),
                class_name.to_owned(),
            ))
        });
        if let Some((name, class)) = captured {
            self.bind_registry_instance_class(name, class);
            return true;
        }
        false
    }

    /// Collision-safe registry instance reporting metadata. Differing class
    /// labels withdraw an ambiguous entry for the complete file. A remaining
    /// entry is presentation only; widget diagnostics independently require
    /// `source_registered_instance_words_at` and the genuine selected schema.
    /// The map supplies no Native receiver, handler or successful construction.
    fn bind_registry_instance_class(&mut self, name: String, class: String) {
        if self.result.ambiguous_instance_names.contains(&name) {
            return;
        }
        match self.result.instance_classes.get(&name) {
            Some(existing) if *existing != class => {
                self.result.instance_classes.remove(&name);
                self.result.ambiguous_instance_names.insert(name);
            }
            _ => {
                self.result.instance_classes.insert(name, class);
            }
        }
    }

    /// Logical source reporting compatibility; Native class identity requires
    /// an original class receipt from the genuine selected command or operand.
    pub(super) fn resolve_user_class_in(&self, name: &str, scope_path: &[usize]) -> Option<String> {
        super::class_hierarchy::resolve_retained_logical_class_name(
            &self.result,
            name,
            &self.command_resolution_namespace(scope_path),
        )
    }

    /// Logical reporting lookup. A propagated value label never issues an
    /// original Native class operand or a positioned class-cell selection.
    fn resolve_user_class_at(&self, name: &str, offset: u32) -> Option<String> {
        super::class_hierarchy::resolve_retained_logical_class_name(
            &self.result,
            name,
            &super::scope::command_resolution_namespace_at(&self.result.global_scope, offset),
        )
    }

    /// Retain a source class label from the genuine setter and constructor.
    /// The canonical original declaration never comes from its printed head.
    fn class_from_constructor_subst(&self, site_offset: u32) -> Option<(String, String)> {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let construction =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                &self.source,
                &self.result,
                site_offset,
            )?;
        let offset = construction.construction().words.first()?.span().start();
        let call = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            offset,
        )?;
        call.constructor_shape(&self.result)?;
        let class = call.class_declaration().source_class(&self.result)?;
        Some((
            std::str::from_utf8(construction.variable_bytes())
                .ok()?
                .to_owned(),
            class.metadata().qualified_name.clone(),
        ))
    }

    /// Logical reporting compatibility for a possible constructor shape.
    /// Registry manufacturer layouts and bounded reported metaclass fallback
    /// metadata supply the model. The caller independently requires positive
    /// retained Logical input; this query supplies no Native class identity,
    /// callable manufacturer, allocation or successful constructor result.
    pub(super) fn class_command_constructs_with(&self, class_q: &str, word: &str) -> bool {
        let Some(grammar) = self.class_definer_grammar(class_q) else {
            // Workspace report metadata supplies the separate Logical model
            // when no local class report determines its family grammar.
            return self.workspace_manufacturer_word(word)
                || (self.workspace_bare_word_classes.contains(class_q)
                    && is_plain_created_name(word));
        };
        if grammar.manufacturer(word).is_some() {
            return self.class_manufacturer_method(class_q, word).is_some();
        }
        if grammar.is_builtin_type_method(word) || !is_plain_created_name(word) {
            return false;
        }
        if self
            .result
            .all_classes
            .get(class_q)
            .is_some_and(|c| c.class_command_fallback.constructs_named_instance())
        {
            return true;
        }
        let Some(meta) = self.metaclass_def(class_q) else {
            return false;
        };
        meta.factory
            .as_ref()
            .is_some_and(|f| f.unknown_binds_instance)
            && !self.metaclass_chain_declares_method(meta, word)
    }

    /// Registry manufacturer layout under the reporting class/export model.
    /// This descriptor supplies source positions, independently of Native
    /// class identity, current manufacturer lookup or actual construction.
    pub(super) fn class_manufacturer_method(
        &self,
        class_q: &str,
        word: &str,
    ) -> Option<&'static tcl_registry::definer::ManufacturerMethod> {
        let method = self.class_definer_grammar(class_q)?.manufacturer(word)?;
        let Some(class) = self.result.all_classes.get(class_q) else {
            return (method.visibility == tcl_registry::definer::MemberVisibility::Exported)
                .then_some(method);
        };
        (!class.class_unexports.contains(word)
            && (method.visibility == tcl_registry::definer::MemberVisibility::Exported
                || class.class_exports.contains(word)))
        .then_some(method)
    }

    /// Whether `word` is a manufacturer method of **any** definer family the
    /// registry models — the fallback for a workspace class whose own
    /// document this analysis has not seen, so its family is unknown here.
    ///
    /// Over-approximate by exactly the union of the families' manufacturer
    /// words (`create` / `new` / `createWithNamespace`), which is what the
    /// `subcmd == "new" || subcmd == "create"` literal this replaces already
    /// assumed — and still registry data, so a new family widens it without
    /// a walker edit.
    fn workspace_manufacturer_word(&self, word: &str) -> bool {
        self.analysis_context()
            .commands()
            .is_manufacturer_method(word)
    }

    /// The definition-body grammar governing `class_q`'s definer family.
    ///
    /// A registry metaclass (`oo::class`, `snit::type`) carries the grammar
    /// on its own spec.  A **user** metaclass has no spec, so the grammar is
    /// the registry metaclass at the root of its superclass chain — recorded
    /// on the factory when the metaclass was written, which is the only place
    /// it is provable.
    pub(super) fn class_definer_grammar(
        &self,
        class_q: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.class_definer_grammar_with_provenance(class_q, false)
    }

    /// Positively retained Logical family advice from an observed class definer.
    /// The current source, lexer and availability inputs must still correspond.
    /// This descriptor supplies neither a Native class token nor construction,
    /// a formal-parameter grammar, method-table closure or dispatch activation.
    pub(super) fn retained_logical_class_definer_grammar(
        &self,
        class_q: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        // naming.diagnostics.retained-logical-class-family
        // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
        if !self.result.allows_retained_logical_declaration_advice() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let context = self.analysis_context();
        if context.context() != input.availability_context()
            || context.commands().snapshot().semantic_key()
                != input
                    .context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
            || self.file_lexer_config() != input.lexer_config()
            || !self.result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                input.lexer_config(),
            )
        {
            return None;
        }
        self.class_definer_grammar_with_provenance(class_q, true)
    }

    fn class_definer_grammar_with_provenance(
        &self,
        class_q: &str,
        require_observed: bool,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let class = self.result.all_classes.get(class_q)?;
        if require_observed
            && class.metaclass_provenance != super::types::MetaclassProvenance::Observed
        {
            return None;
        }
        if let Some(grammar) = self.definition_grammar(&class.metaclass) {
            return Some(grammar);
        }
        let meta = self.metaclass_def(class_q)?;
        if require_observed
            && meta.metaclass_provenance != super::types::MetaclassProvenance::Observed
        {
            return None;
        }
        self.definition_grammar(&meta.factory.as_ref()?.root_metaclass)
    }

    /// The recorded `ClassDef` of `class_q`'s metaclass, when the metaclass
    /// is itself a class this document knows.
    ///
    /// Resolved through the shared owner-aware class-name resolver, so a
    /// bare metaclass word resolves exactly as a `superclass` word in the
    /// same position would — and abstains on an ambiguous tail rather than
    /// picking one.
    pub(super) fn metaclass_def(&self, class_q: &str) -> Option<&super::types::ClassDef> {
        let class = self.result.all_classes.get(class_q)?;
        let resolved = super::class_hierarchy::resolve_class_lookup(
            class.metaclass_lookup.as_ref()?,
            &self.result.all_classes,
        )?;
        self.result.all_classes.get(&resolved)
    }

    /// Whether `method` is declared as an instance method anywhere in
    /// `meta`'s own superclass chain — the members a **class command**
    /// dispatch really finds, because a class is an instance of its
    /// metaclass.
    ///
    /// Bounded by the recorded class count: every class is visited once.
    fn metaclass_chain_declares_method(&self, meta: &super::types::ClassDef, method: &str) -> bool {
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: Vec<&super::types::ClassDef> = vec![meta];
        while let Some(class) = queue.pop() {
            if !visited.insert(class.qualified_name.clone()) {
                continue;
            }
            if class.methods.contains_key(method) {
                return true;
            }
            for parent in &class.superclasses {
                if let Some(resolved) = class
                    .relation_lookups
                    .get(parent)
                    .and_then(Option::as_ref)
                    .and_then(|lookup| {
                        super::class_hierarchy::resolve_class_lookup(
                            lookup,
                            &self.result.all_classes,
                        )
                    })
                    && let Some(def) = self.result.all_classes.get(&resolved)
                {
                    queue.push(def);
                }
            }
        }
        false
    }

    /// Retain the genuine setter and scalar-variable constructor geometry.
    /// The later value model can supply reporting labels for explicitly Logical
    /// input. Source geometry and constant values do not establish Native class
    /// selection, constructor execution or an allocated result.
    fn record_pending_instance_class_site(&mut self, site_offset: u32) {
        if self.class_from_constructor_subst(site_offset).is_some() {
            return;
        }
        let Some(construction) =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                &self.source,
                &self.result,
                site_offset,
            )
        else {
            return;
        };
        let Some((class_var, manufacturer_word, span)) =
            class_var_head_constructor_subst(&construction)
        else {
            return;
        };
        let Ok(target_name) = std::str::from_utf8(construction.variable_bytes()) else {
            return;
        };
        self.pending_instance_class_sites
            .push(super::state::PendingInstanceClassSite {
                class_var,
                manufacturer_word,
                span,
                target_name: target_name.to_owned(),
            });
    }

    /// Retain class-variable constructor labels for explicitly Logical input.
    /// Constant contributors supply reporting values, not genuine original
    /// Native class operands, current source-cell occupancy or allocations.
    /// Native and hosted consumers require their own selected class-instance
    /// carrier and cannot recover it by parsing a propagated value label.
    pub(in crate::analyser) fn settle_pending_instance_class_sites(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.pending_instance_class_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.pending_instance_class_sites);
        if !self
            .result
            .resolved_input
            .as_ref()
            .is_some_and(super::input::ResolvedAnalysisInput::has_logical_source_name_context)
        {
            return;
        }
        let config = self.lexer_config();
        for site in &sites {
            // A write trace can mutate the variable at any read — see
            // `settle_const_dispatches`'s identical guard.
            if cu.ir_module.has_dynamic_variable_trace
                || cu.ir_module.traced_variables.contains(&site.class_var)
            {
                continue;
            }
            let fu = cu.function_unit_at(site.span.start());
            let Some(contributors) = crate::value_provenance::const_contributors(
                fu,
                site.span.start(),
                &site.class_var,
                config,
            ) else {
                continue;
            };
            let mut resolved: Option<String> = None;
            for c in &contributors {
                let value = c.value.trim();
                if value.is_empty() || crate::naming::is_dynamic_word(value) {
                    resolved = None;
                    break;
                }
                let Some(qc) = self.resolve_user_class_at(value, site.span.start()) else {
                    resolved = None;
                    break;
                };
                if !self.class_command_constructs_with(&qc, &site.manufacturer_word) {
                    resolved = None;
                    break;
                }
                match &resolved {
                    None => resolved = Some(qc),
                    Some(existing) if *existing != qc => {
                        resolved = None;
                        break;
                    }
                    Some(_) => {}
                }
            }
            if let Some(class) = resolved {
                self.result
                    .instance_classes
                    .insert(site.target_name.clone(), class);
            }
        }
    }
}

/// Retain an exact scalar-variable constructor head and static selector from
/// the shared original setter/substitution carrier. Braced variable names and
/// escaped selectors follow their lexical/name owners. Composite and array
/// heads decline. The source span is the authentic reference extent; no class
/// identity, variable value or successful result is issued here.
pub(in crate::analyser) fn class_var_head_constructor_subst(
    construction: &crate::registry_invocation::OriginalSourceHandleConstruction,
) -> Option<(String, String, Span)> {
    let words = &construction.construction().words;
    let head = words.first()?;
    if head.group().expand {
        return None;
    }
    let arena = head.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Variable { name, index: None } = part.part else {
        return None;
    };
    let class_var = std::str::from_utf8(arena.bytes(name)?).ok()?.to_owned();
    let protocol = construction
        .setter()
        .dialect()?
        .native_source_string_protocol()?;
    let captured =
        tcl_registry::native_compiler_words::NativeCompilerWords::capture(words, protocol).ok()?;
    Some((
        class_var,
        std::str::from_utf8(captured.literal(1)?).ok()?.to_owned(),
        part.span,
    ))
}

/// Whether `name` is a concrete, bindable instance-command name in a
/// `CLASS create NAME` construct — a plain word the analyser can register.
/// Excludes the auto-name token (`%AUTO%`), computed names (`$v`, `[…]`), and
/// any word carrying list / substitution metacharacters.
fn is_plain_created_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('%')
        && !name.contains(['$', '[', ']', '{', '}', '(', ')', ' ', '"'])
}

/// Descend a ``Cmd`` (``[…]``) substitution token and collect every
/// inner command's head as ``(name, head_span)``, recursing into nested
/// ``[...]`` substitutions and registry-resolved body arguments.
///
/// The token is descended into a child CST ([`descend_token`]) and the
/// inner script segmented; each command is then handled by
/// [`record_command_invocations`].  Spans are absolute (the descent
/// anchors the child tree at the substitution's position).
/// Statically-known argument count of a segmented command for the cross-file
/// arity check: `None` when any argument word is `{*}`-expanded (runtime count
/// unknown), else the literal argument count (`argv` minus the head).
fn segment_argc(seg: &SegmentedCommand) -> Option<usize> {
    if seg
        .expand_word
        .as_ref()
        .is_some_and(|e| e.iter().skip(1).any(|&x| x))
    {
        return None;
    }
    Some(seg.argv.len().saturating_sub(1))
}

/// The segment's first argument word, as an ensemble-subcommand candidate
/// `(text, span)` — `None` when there is no such word, it's `{*}`-expanded
/// (so the runtime subcommand isn't known statically), or it's otherwise a
/// dynamic word (the nested-`[...]` counterpart of the top-level check in
/// `record_ensemble_subcommand_invocation`).
fn ensemble_subcommand_candidate(seg: &SegmentedCommand) -> Option<(String, Span)> {
    let expanded = seg
        .expand_word
        .as_ref()
        .and_then(|e| e.get(1))
        .copied()
        .unwrap_or(false);
    if expanded {
        return None;
    }
    let sub = seg.texts.get(1)?;
    if crate::naming::is_dynamic_word(sub) {
        return None;
    }
    let span = seg.argv.get(1)?.span;
    Some((sub.clone(), span))
}

fn collect_substitution_heads(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        record_command_invocations(sm, analysis, &seg, config, out, expr_tokens);
    }
}

fn selected_rule_procedure_operand(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<usize> {
    schema.authored_source_rule_procedure_operand()
}

fn nested_source_words(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    segment: &SegmentedCommand,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    crate::registry_invocation::source_structure::source_registry_words(
        sm.source(),
        analysis,
        segment,
    )
}

fn nested_source_bodies(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    segment: &SegmentedCommand,
    config: LexerConfig,
    purpose: crate::registry_invocation::OriginalSourceScriptPurpose,
) -> Vec<SegmentedCommand> {
    let Some(words) = nested_source_words(sm, analysis, segment) else {
        return Vec::new();
    };
    let Some(input) = &analysis.resolved_input else {
        return Vec::new();
    };
    let context = input.context_registry();
    if words.with_source_schema(&context, |schema| {
        schema.authored_source_definition_body_grammar().is_some()
            || schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
    }) != Some(false)
    {
        return Vec::new();
    }
    let mut commands = Vec::new();
    for body in words.source_script_bodies_for(&context, purpose) {
        let span = body.content_span();
        if let Some(text) = sm.source().get(span.as_range()) {
            commands.extend(crate::segmenter::segment_commands_with_offset_and_config(
                text,
                span.start(),
                config,
            ));
        }
    }
    commands
}

fn collected_original_callback(
    prefix: crate::command_binding::OriginalCallbackPrefix,
) -> Option<CollectedHead> {
    let (name, span) = prefix.reported_source_head()?;
    Some(CollectedHead {
        name: name.to_owned(),
        span,
        argc: None,
        callback: Some(prefix),
        ensemble: None,
    })
}

fn nested_source_prefixes(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    segment: &SegmentedCommand,
    config: LexerConfig,
) -> Vec<CollectedHead> {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let Some(input) = &analysis.resolved_input else {
        return Vec::new();
    };
    let context = input.context_registry();
    let Some(offset) = segment.argv.first().map(|head| head.span.start()) else {
        return Vec::new();
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(sm, config, segment);
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    realm.stamp_original_tokens(&mut tokens);
    if let Some(words) = nested_source_words(sm, analysis, segment) {
        return words.with_source_schema(&context, |schema| {
            let facts = schema.facts();
            let Some(dialect) = schema.words.arguments().dialect() else { return Vec::new(); };
            let mut prefixes = Vec::new();
            for (argument, appended) in schema.authored_source_command_prefix_arguments().unwrap_or_default() {
                let Some(native_input) = words.original_argument_value_input(argument) else { continue; };
                let Some(word) = native_input.original_word_key().map(crate::signature_scan::scope::SignatureSourceNameKey::original_word) else { continue; };
                let original = match argument.checked_add(1).and_then(|ordinal| words.origins().get(ordinal)) {
                    Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) => tokens.source_binding.as_ref()
                        .and_then(|binding| binding.original_callback_prefix_in_context(&tokens, *written, &context)),
                    _ => None,
                };
                let prefix = original.or_else(|| crate::command_binding::OriginalCallbackPrefix::from_original_static_operand(
                    word, &facts, argument, dialect));
                let prefix = prefix.map(|prefix| {
                    crate::registry_invocation::source_structure::source_callback_prefix_at(
                        sm.source(), analysis, offset, &prefix,
                    ).unwrap_or(prefix)
                });
                if let Some(head) = prefix.filter(|prefix| prefix.appended_arity() == Some(appended))
                    .and_then(collected_original_callback) { prefixes.push(head); }
            }
            prefixes
        }).unwrap_or_default();
    }
    let Some(instance) =
        crate::registry_invocation::source_structure::source_registered_instance_words_at(
            sm.source(),
            analysis,
            offset,
        )
    else {
        return Vec::new();
    };
    instance.with_source_schema(&context, |schema| {
        let facts = schema.facts();
        let Some(dialect) = schema.words.arguments().dialect() else { return Vec::new(); };
        schema.authored_source_command_prefix_arguments().unwrap_or_default().into_iter().filter_map(|(argument, appended)| {
            instance.argument_input(argument)?.native_input()?;
            let prefix = crate::command_binding::OriginalCallbackPrefix::from_original_static_operand(
                instance.argument_word(argument)?, &facts, argument, dialect)?;
            (prefix.appended_arity() == Some(appended)).then_some(())?;
            let prefix = crate::registry_invocation::source_structure::source_callback_prefix_at(
                sm.source(), analysis, offset, &prefix,
            ).unwrap_or(prefix);
            collected_original_callback(prefix)
        }).collect()
    }).unwrap_or_default()
}

fn record_command_invocations(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    seg: &SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if let (Some(&head), Some(name)) = (seg.argv.first(), seg.texts.first())
        && !name.is_empty()
    {
        out.push(CollectedHead {
            name: name.clone(),
            span: head.span,
            argc: segment_argc(seg),
            callback: None,
            ensemble: ensemble_subcommand_candidate(seg),
        });
    }
    out.extend(nested_source_prefixes(sm, analysis, seg, config));
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_heads(sm, analysis, *tok, config, out, expr_tokens);
        }
    }
    for inner in nested_source_bodies(
        sm,
        analysis,
        seg,
        config,
        crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
    ) {
        record_command_invocations(sm, analysis, &inner, config, out, expr_tokens);
    }
    let Some(words) = nested_source_words(sm, analysis, seg) else {
        return;
    };
    for (index, role) in words.written_argument_roles() {
        if role == ArgRole::Expr
            && let Some(&tok) = seg.argv.get(index + 1)
        {
            expr_tokens.push(tok);
            collect_expr_substitutions(sm, analysis, tok, config, out, expr_tokens);
        }
    }
}

fn collect_expr_substitutions(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    expr_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if expr_tok.kind != TokenType::Str || sm.token_text(expr_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, expr_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        for tok in &seg.all_tokens {
            if tok.kind == TokenType::Cmd {
                collect_substitution_heads(sm, analysis, *tok, config, out, expr_tokens);
            }
        }
    }
}

fn collect_substitution_segments(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
) {
    collect_substitution_segments_at(sm, analysis, cmd_tok, config, out, 0);
}

fn collect_substitution_segments_at(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        collect_segment_recursive(sm, analysis, seg, config, out, depth);
    }
}

fn collect_segment_recursive(
    sm: &SourceMap<'_>,
    analysis: &super::types::AnalysisResult,
    seg: SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.push(seg);
        return;
    }
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_segments_at(sm, analysis, *tok, config, out, depth + 1);
        }
    }
    for inner in nested_source_bodies(
        sm,
        analysis,
        &seg,
        config,
        crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
    ) {
        collect_segment_recursive(sm, analysis, inner, config, out, depth + 1);
    }
    out.push(seg);
}

/// Top-level ``[...]`` command-substitution regions in `text`, as
/// `(inner_byte_offset, inner_text)` — the script *inside* the brackets and
/// the offset of its first byte within `text`.  ``\\[`` / ``\\]`` escapes are
/// honoured.  Only the *outermost* substitutions are returned; the caller's
/// segment recursion descends any nested `[...]`.  Used to reach `[...]`
/// embedded in a quoted / bareword word argument (`log "got [HTTP::uri]"`),
/// which the bare-`Cmd`-token walk misses.
///
/// Braces are **not** treated as opaque here: this only runs on `Esc` words
/// (quoted / bareword / compound).  A braced *word* is a `Str` token (handled
/// elsewhere and excluded by the caller); inside a quoted or bareword context
/// `{` / `}` are ordinary characters that do *not* suppress substitution, so
/// `log "got { [HTTP::uri] }"` still executes — and must still be scanned.
fn top_level_cmd_subst_regions(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth == 0 && j <= bytes.len() {
                    out.push((inner_start, &text[inner_start..j]));
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    out
}

/// Walk `text` looking for ``[cmd args...]`` command
/// substitutions and return the head name + the offset of the
/// head's first byte within `text` for each substitution found.
/// Nested substitutions are reported in depth-first order
/// (outer first, then inner).  Braced regions are skipped
/// opaquely; backslash-escaped ``\\[`` / ``\\]`` are skipped.
///
/// Returns ``(name, byte_offset_in_text)`` pairs.  The caller
/// adds the enclosing token's source-span start to obtain an
/// absolute offset.
pub(crate) fn scan_nested_command_heads(text: &str) -> Vec<(String, u32)> {
    // Entry point: the outermost word's raw text is bracket-nesting depth 0
    // (the recursion cap lives in [`scan_nested_command_heads_at`]).
    scan_nested_command_heads_at(text, 0)
}

fn scan_nested_command_heads_at(text: &str, rec_depth: u32) -> Vec<(String, u32)> {
    // Native-stack safety net: this self-recurses once per
    // nested `[…]` substitution inside a single word's raw text — a genuinely
    // unbounded axis. Past the cap, return what's been found so far: nested
    // heads buried deeper than the cap go unreported, never a crash.
    // (`rec_depth` is named apart from the local `[`/`{`-matching `depth`
    // counters below, which track bracket balance, not native recursion.)
    if MAX_BRACKET_TEXT_DEPTH.exceeded(rec_depth) {
        return Vec::new();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Skip braced region opaquely.
                let mut depth = 1i32;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\\' if i + 1 < bytes.len() => i += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'[' => {
                // Find the matching closing ``]`` honouring
                // nesting + backslash escapes.
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    j += 1;
                }
                if depth == 0 && j < bytes.len() {
                    let inner = &text[inner_start..j];
                    let inner_start_u32 = u32::try_from(inner_start)
                        .expect("byte offset fits in u32 for in-memory source");
                    if let Some((name, head_offset_in_inner)) = first_command_head(inner) {
                        let abs_offset = inner_start_u32
                            + u32::try_from(head_offset_in_inner)
                                .expect("inner offset fits in u32 for in-memory source");
                        out.push((name.to_string(), abs_offset));
                    }
                    // Recurse into the inner text — nested ``[...]``
                    // substitutions inside this one also produce
                    // invocations.
                    for (name, off_in_inner) in scan_nested_command_heads_at(inner, rec_depth + 1) {
                        out.push((name, inner_start_u32 + off_in_inner));
                    }
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    out
}

/// The argument count a call site advertises for cross-file arity checking.
///
/// A `{*}`-expanded argument makes the runtime count unknown, so the answer
/// is `None` and arity checking skips conservatively.
fn call_arg_count(args: &[String], arg_expand_in: &[bool]) -> Option<usize> {
    if arg_expand_in.iter().skip(1).copied().any(|e| e) {
        None
    } else {
        Some(args.len())
    }
}

/// Find the first command-head token in `text` (skipping
/// leading whitespace and comments) and return its ``(name,
/// offset)`` pair.  Conservative — any non-bareword leading
/// token returns ``None``.
fn first_command_head(text: &str) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b';' {
            i += 1;
            continue;
        }
        // Skip comment lines if at the start of a logical command.
        if c == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b';'
            || c == b'['
            || c == b']'
        {
            break;
        }
        i += 1;
    }
    if i == start {
        return None;
    }
    let head = &text[start..i];
    // Reject heads that look like substitution / quoting markers
    // (``$foo``, ``"abc"``, ``{...}``) — these aren't command
    // names.
    if head.starts_with('$') || head.starts_with('"') || head.starts_with('{') {
        return None;
    }
    Some((head, start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{Span, TokenType};

    /// `if {1} { if {1} { ... } }`, `depth` levels deep, wrapped in a `proc`
    /// body.
    fn nested_if_source(depth: u32) -> String {
        let mut source = String::from("proc deepnest {} {\n");
        for _ in 0..depth {
            source.push_str("if {1} {\n");
        }
        for _ in 0..depth {
            source.push_str("}\n");
        }
        source.push_str("}\n");
        source
    }

    /// Run `analyse` on a dedicated thread with a generous stack.
    ///
    /// `cargo test` runs each `#[test]` on its own thread with the platform
    /// default stack size (~2 MiB on Linux) — the same undersized budget
    /// that a deep walk overflows (Tokio's default
    /// worker-thread stack is the same size). A test that walks source
    /// nested past [`MAX_BODY_DEPTH`] needs the same generous, explicit
    /// stack production code now gets via `tokio::runtime::Builder::
    /// thread_stack_size` (`tcl-lsp-server`/`tcl-mcp`) and `std::thread::
    /// Builder::stack_size` (the `tcl` CLI) — otherwise the test harness
    /// itself hits the bug this suite exists to catch.
    fn analyse_on_big_stack(source: String, dialect: &str) -> super::super::types::AnalysisResult {
        let dialect = dialect.to_owned();
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || Analyser::new().analyse(&source, &dialect))
            .expect("spawn big-stack test thread")
            .join()
            .expect("analyse on big-stack thread panicked")
    }

    #[test]
    fn depth_exactly_at_cap_emits_no_e207() {
        // The wrapping `proc` body is itself one level of `body_depth`, so
        // `MAX_BODY_DEPTH - 1` nested `if`s is what brings body_depth to
        // exactly `MAX_BODY_DEPTH`.
        let source = nested_if_source(MAX_BODY_DEPTH.0 - 1);
        let res = analyse_on_big_stack(source, "tcl9.0");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
            "depth == MAX_BODY_DEPTH must not trip the cap: {:?}",
            res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn depth_one_past_cap_emits_e207_exactly_once() {
        let source = nested_if_source(MAX_BODY_DEPTH.0);
        let res = analyse_on_big_stack(source, "tcl9.0");
        let e207_count = res
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E207)
            .count();
        assert_eq!(
            e207_count, 1,
            "depth == MAX_BODY_DEPTH + 1 must trip the cap exactly once, got {e207_count}"
        );
    }

    #[test]
    fn depth_far_past_cap_still_emits_e207_exactly_once() {
        // Not flooded: the cap trips at the same nesting level every time
        // (once body_depth cannot go higher), so it must still be exactly
        // one diagnostic at 2000 levels, not one per level past the cap.
        let source = nested_if_source(2000);
        let res = analyse_on_big_stack(source, "tcl9.0");
        let e207_count = res
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E207)
            .count();
        assert_eq!(
            e207_count, 1,
            "2000 levels must still trip the cap exactly once, got {e207_count}"
        );
    }

    #[test]
    fn shallow_nesting_never_emits_e207() {
        // False-positive guard: ordinary, hand-written nesting must never
        // draw E207.
        let source = nested_if_source(10);
        let mut a = Analyser::new();
        let res = a.analyse(&source, "tcl9.0");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
            "shallow nesting must never emit E207: {:?}",
            res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn resolve_analyser_hook_uses_registry_rooted_resolution() {
        use tcl_registry::hooks::AnalyserHookId as H;
        let a = Analyser::new();
        let args = |words: &[&str]| words.iter().map(ToString::to_string).collect::<Vec<_>>();

        // Unstamped head: no handler family.
        assert_eq!(a.resolve_analyser_hook("puts", &args(&["hi"])), None);
        // Command-level stamp.
        assert_eq!(
            a.resolve_analyser_hook("proc", &args(&["p", "a", "b"])),
            Some(H::Proc)
        );
        // Subcommand-level stamps pick the family per subcommand word…
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["eval", "ns", "{}"])),
            Some(H::NamespaceEval)
        );
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["import", "::t::*"])),
            Some(H::NamespaceImport)
        );
        // …and an unstamped subcommand of a stamped command resolves
        // nothing (`namespace which`, `dict set`).
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["which", "x"])),
            None
        );
        assert_eq!(
            a.resolve_analyser_hook("dict", &args(&["set", "d", "k", "v"])),
            None
        );
        assert_eq!(
            a.resolve_analyser_hook("dict", &args(&["with", "$d", "{}"])),
            Some(H::DictWith)
        );
        // A namespaced registry spelling and its rooted form resolve
        // identically through the registry owner.
        assert_eq!(
            a.resolve_analyser_hook("tcl::OptProc", &args(&["p", "{a}", "{}"])),
            Some(H::OptProc)
        );
        assert_eq!(
            a.resolve_analyser_hook("::tcl::OptProc", &args(&["p", "{a}", "{}"])),
            Some(H::OptProc)
        );
        // Rootable global commands have the same exact binding and hook with
        // or without their explicit global qualifier.
        assert_eq!(
            a.resolve_analyser_hook("::proc", &args(&["p", "a", "b"])),
            Some(H::Proc)
        );
        assert_eq!(
            a.resolve_analyser_hook("set", &args(&["name", "value"])),
            a.resolve_analyser_hook("::set", &args(&["name", "value"]))
        );
    }

    /// A Tk widget constructor is syntactically identical to a tcllib
    /// naming factory (`struct::graph g`) — a bareword `ttk::treeview .t`
    /// retains a conditional `.t` class label through the shared original
    /// source factory receipt, without publishing successful creation.
    #[test]
    fn bareword_widget_constructor_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("ttk::treeview .t\n.t instate {selected} {}\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get(".t").map(String::as_str),
            Some("ttk::treeview"),
            "instance_classes: {:?}",
            res.instance_classes
        );
        assert!(res.created_instance_commands.is_empty());
        assert!(
            crate::registry_invocation::source_structure::source_command_publication_at(
                "ttk::treeview .t\n.t instate {selected} {}\n",
                &res,
                0
            )
            .is_some()
        );
    }

    /// The `set w [ctor .path]` return-value-capture shape resolves through
    /// the shared source handle-class receipt and retains a class label.
    #[test]
    fn var_captured_widget_constructor_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("set lb [listbox .l]\n$lb curselection\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get("lb").map(String::as_str),
            Some("listbox"),
            "instance_classes: {:?}",
            res.instance_classes
        );
    }

    /// A plain widget with no modelled subcommands (`button`) still binds
    /// an instance class (useful for definition/references/W123
    /// suppression) even though it has no `object_class` to dispatch
    /// methods against.
    #[test]
    fn simple_widget_without_subcommands_still_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("button .b -text hi\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get(".b").map(String::as_str),
            Some("button"),
            "instance_classes: {:?}",
            res.instance_classes
        );
    }

    #[test]
    fn interp_create_original_name_positions_keep_option_width_and_terminators() {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        for (source, name) in [
            ("interp create -safe sandbox", "sandbox"),
            ("interp create -- sandbox", "sandbox"),
            ("interp create -- -safe", "-safe"),
            ("interp create -safe -- sandbox", "sandbox"),
            ("interp create sandbox", "sandbox"),
        ] {
            let result = Analyser::new().analyse(source, "tcl9.0");
            let publication =
                crate::registry_invocation::source_structure::source_command_publication_at(
                    source, &result, 0,
                )
                .unwrap();
            assert_eq!(publication.name_bytes(), name.as_bytes());
            assert_eq!(
                publication.kind(),
                tcl_registry::AuthoredSourceCommandPublicationKind::Command
            );
            assert!(
                result.created_instance_commands.is_empty(),
                "a source naming layout is not successful creation"
            );
        }
    }

    #[test]
    fn interp_create_safe_with_literal_name_suppresses_w123_on_later_call() {
        // TP — the user-visible symptom: with the name unrecorded, every
        // literal call to the interpreter's own object command raises a
        // false-positive W123 "unknown command".
        let src = "interp create -safe sandbox\nsandbox eval {set x 1}\n";
        assert!(
            !has_code(src, "tcl9.0", "W123"),
            "a literal call to a `interp create -safe`-created name must \
             not raise W123: {:?}",
            diag_codes(src, "tcl9.0")
        );
    }

    #[test]
    fn interp_create_with_no_name_records_nothing() {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        let source = "interp create -safe";
        let result = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            crate::registry_invocation::source_structure::source_command_publication_at(
                source, &result, 0
            )
            .is_none()
        );
        assert!(result.created_instance_commands.is_empty());
    }

    #[test]
    fn scan_nested_command_heads_simple() {
        let out = scan_nested_command_heads("[helper $x]");
        assert_eq!(out, vec![("helper".to_string(), 1)]);
    }

    #[test]
    fn scan_nested_command_heads_nested() {
        // [outer [inner $x]]
        let out = scan_nested_command_heads("[outer [inner $x]]");
        assert_eq!(
            out,
            vec![("outer".to_string(), 1), ("inner".to_string(), 8)]
        );
    }

    /// Depth coverage: `scan_nested_command_heads` self-recurses once per
    /// nested `[…]` substitution inside a single word's raw text, and
    /// `collect_segment_recursive` / `collect_substitution_segments` mutually
    /// recurse the same way. Both axes are unbounded without their own caps,
    /// independent of the statement-tree `MAX_BODY_DEPTH` cap, and overflow the
    /// native stack (SIGABRT) in the low thousands of levels on a 2 MiB
    /// thread. 3000 is past that crash range and past `MAX_BRACKET_TEXT_DEPTH`
    /// (256); the assertion is that each returns.
    #[test]
    fn deeply_nested_command_head_scans_survive() {
        // `[a [a [a … [a x] … ]]]` nested bracket substitutions drive the
        // direct Tier 1B text scanner.
        let mut brackets = "x".to_owned();
        for _ in 0..3000 {
            brackets = format!("[a {brackets}]");
        }
        let _ = scan_nested_command_heads(&brackets);

        // `a [a [a … [x] … ]]` — each level is a literal-headed command whose
        // argument holds the next `[…]` substitution — drives the
        // `collect_segment_recursive` / `collect_substitution_segments` mutual
        // recursion directly (isolated from the full analyser pipeline).
        let mut nested = "x".to_owned();
        for _ in 0..3000 {
            nested = format!("a [{nested}]");
        }
        let config = LexerConfig::for_dialect("tcl8.6");
        let sm = SourceMap::new(&nested);
        let analysis = crate::analyser::types::AnalysisResult::default();
        let mut out = Vec::new();
        for seg in crate::segmenter::segment_commands_with_offset_and_config(&nested, 0, config) {
            collect_segment_recursive(&sm, &analysis, seg, config, &mut out, 0);
        }
    }

    #[test]
    fn scan_nested_command_heads_inside_quoted_string() {
        // "got [count $items]" — quotes don't interfere with [
        // ``count`` starts at byte 6 (after ``"got [``).
        let out = scan_nested_command_heads("\"got [count $items]\"");
        assert_eq!(out, vec![("count".to_string(), 6)]);
    }

    #[test]
    fn scan_nested_command_heads_skips_braced_regions() {
        // Braced regions are opaque — `[` inside `{...}` doesn't count.
        // ``real`` starts at byte 15 (after ``{[not_a_cmd]} [``).
        let out = scan_nested_command_heads("{[not_a_cmd]} [real]");
        assert_eq!(out, vec![("real".to_string(), 15)]);
    }

    #[test]
    fn scan_nested_command_heads_skips_backslash_escape() {
        // \[foo\] is a literal pair, not a substitution.
        // ``bar`` starts at byte 9 (after ``\\[foo\\] [``).
        let out = scan_nested_command_heads("\\[foo\\] [bar]");
        assert_eq!(out, vec![("bar".to_string(), 9)]);
    }

    #[test]
    fn scan_nested_command_heads_no_match_for_pure_var_head() {
        // [$cmd args] — head is a variable substitution, not a name
        let out = scan_nested_command_heads("[$cmd args]");
        assert_eq!(out, [] as [(std::string::String, u32); 0]);
    }

    #[test]
    fn scan_nested_command_heads_no_match_for_unclosed() {
        // Unclosed [ — returns nothing (recovery is segmenter's job)
        let out = scan_nested_command_heads("[lindex $x 0");
        assert_eq!(out, [] as [(std::string::String, u32); 0]);
    }

    #[test]
    fn scan_nested_command_heads_two_independent_substs() {
        // [a $x] [b $y]
        let out = scan_nested_command_heads("[a $x] [b $y]");
        assert_eq!(out, vec![("a".to_string(), 1), ("b".to_string(), 8)]);
    }

    fn esc_tok(span: Span) -> Token {
        Token::new(TokenType::Esc, span)
    }

    fn str_tok(span: Span) -> Token {
        Token {
            kind: TokenType::Str,
            span,
            content_offset: 1,
            in_quote: false,
        }
    }

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    #[test]
    fn process_set_defines_variable() {
        let mut a = Analyser::new();
        a.process_command(
            &["set".to_string(), "x".to_string(), "1".to_string()],
            &[
                esc_tok(span(0, 3)),
                esc_tok(span(4, 5)),
                esc_tok(span(6, 7)),
            ],
            &[true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn process_proc_records_at_global() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "proc".to_string(),
                "foo".to_string(),
                "a b".to_string(),
                "set x $a".to_string(),
            ],
            &[
                esc_tok(span(0, 4)),
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 25)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert!(a.result.all_procs.contains_key("::foo"));
    }

    #[test]
    fn process_namespace_eval_opens_scope() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "namespace".to_string(),
                "eval".to_string(),
                "ns1".to_string(),
                String::new(),
            ],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 21)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
    }

    #[test]
    fn process_foreach_defines_loop_var() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "foreach".to_string(),
                "i".to_string(),
                "{1 2 3}".to_string(),
                "puts $i".to_string(),
            ],
            &[
                esc_tok(span(0, 7)),
                esc_tok(span(8, 9)),
                str_tok(span(10, 17)),
                str_tok(span(18, 28)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("i"));
    }

    #[test]
    fn process_global_defines_each_name() {
        let mut a = Analyser::new();
        a.process_command(
            &["global".to_string(), "x".to_string(), "y".to_string()],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 8)),
                esc_tok(span(9, 10)),
            ],
            &[true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn process_unknown_command_silently_no_op() {
        let mut a = Analyser::new();
        a.process_command(
            &["my_unknown_command".to_string(), "arg".to_string()],
            &[esc_tok(span(0, 18)), esc_tok(span(19, 22))],
            &[true, true],
            &[],
            &[],
        );
        // No handler matched; no procs, vars, classes, or aliases
        // recorded.
        assert_eq!(a.result.all_procs.len(), 0);
        assert_eq!(a.result.global_scope.variables.len(), 0);
    }

    #[test]
    fn process_empty_argv_is_no_op() {
        let mut a = Analyser::new();
        a.process_command(&[], &[], &[], &[], &[]);
        // No panic, no state mutation.
    }

    #[test]
    fn process_interp_alias_records_target() {
        let mut a = Analyser::new();
        // The handler reads the registry's one command-table transition
        // vocabulary, so a bare harness has to carry a registry the way a
        // real walk does.
        a.registry = Some(std::sync::Arc::clone(
            tcl_registry::model::ingress::static_context_for("tcl").commands(),
        ));
        a.process_command(
            &[
                "interp".to_string(),
                "alias".to_string(),
                String::new(),
                "myset".to_string(),
                String::new(),
                "set".to_string(),
            ],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 12)),
                str_tok(span(13, 15)),
                esc_tok(span(16, 21)),
                str_tok(span(22, 24)),
                esc_tok(span(25, 28)),
            ],
            &[true, true, true, true, true, true],
            &[],
            &[],
        );
        assert!(a.command_aliases.contains_key("::myset"));
    }

    fn diag_codes(source: &str, dialect: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        let res = a.analyse(source, dialect);
        res.diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn has_code(source: &str, dialect: &str, code: &str) -> bool {
        diag_codes(source, dialect).iter().any(|(c, _)| c == code)
    }

    /// A stray clause word is reported against the command whose clause
    /// grammar owns it — the registry's keyword table, not a list here.
    #[test]
    fn orphaned_keywords_name_the_grammar_that_owns_them() {
        use tcl_registry::clause_grammar::owner_of_keyword;
        assert_eq!(owner_of_keyword("else"), Some("if"));
        assert_eq!(owner_of_keyword("elseif"), Some("if"));
        assert_eq!(owner_of_keyword("then"), Some("if"));
        assert_eq!(owner_of_keyword("on"), Some("try"));
        assert_eq!(owner_of_keyword("trap"), Some("try"));
        assert_eq!(owner_of_keyword("finally"), Some("try"));
        assert_eq!(owner_of_keyword("set"), None);
    }

    #[test]
    fn w125_fires_for_orphaned_else() {
        // A misplaced newline split the `if` — `else` lands as a standalone
        // command with no parent.
        assert!(has_code(
            "if {$x} {\n  puts hi\n}\nelse {\n  puts bye\n}",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn w125_quiet_for_attached_else() {
        // Properly attached `} else {` never segments `else` as a command.
        assert!(!has_code(
            "if {$x} {\n  puts hi\n} else {\n  puts bye\n}",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn w125_suppressed_when_user_proc_shadows_keyword() {
        // A user proc named `finally` shadows the keyword — no warning.
        assert!(!has_code(
            "proc finally {} { return }\nfinally",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_in_event() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_in_command_substitution() {
        // A direct proc call nested inside a `[…]` substitution still bypasses
        // `call`, so IRULE5005 must reach it via the nested-segment walk.
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set x [helper] }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_quiet_with_call_prefix() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { call helper }";
        assert!(!has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn malformed_irules_proc_is_not_registered_or_walked() {
        let mut analyser = Analyser::new();
        let result = analyser.analyse(
            "proc malformed {} { pool /Common/inert } extra\n\
             when HTTP_REQUEST { call malformed }",
            "f5-irules",
        );
        assert!(!result.all_procs.contains_key("::malformed"));
        assert!(result.command_invocations.iter().all(|call| {
            call.name != "pool" && call.resolved_qualified_name.as_deref() != Some("::pool")
        }));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_from_proc() {
        let src = "proc helper {} { return 1 }\nproc caller {} { helper }\nwhen RULE_INIT { call caller }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_in_proc_command_substitution() {
        let src = "proc helper {} { return 1 }\nproc caller {} { set x [helper] }\nwhen RULE_INIT { call caller }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_quiet_for_call_prefix_from_proc() {
        let src = "proc helper {} { return 1 }\nproc caller {} { call helper }\nwhen RULE_INIT { call caller }";
        assert!(!has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5007_rejects_direct_or_call_prefixed_proc_at_top_level() {
        for invocation in ["helper", "call helper"] {
            let src = format!("proc helper {{}} {{ return 1 }}\n{invocation}");
            assert!(has_code(&src, "f5-irules", "IRULE5007"));
            assert!(!has_code(&src, "f5-irules", "IRULE5005"));
        }
    }

    #[test]
    fn irule5005_quiet_in_plain_tcl_dialect() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
        assert!(!has_code(src, "tcl", "IRULE5005"));
    }

    #[test]
    fn irule5005_carries_call_rewrite_fix() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper x y }";
        let mut a = Analyser::new();
        let res = a.analyse(src, "f5-irules");
        let d = res
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::Irule5005)
            .expect("IRULE5005 expected");
        assert!(d.message.contains("call helper x y"));
        assert_eq!(d.fixes.len(), 1);
        assert_eq!(d.fixes[0].new_text, "call helper");
    }

    /// A math-function call in an expression resolves to the
    /// `::tcl::mathfunc::<name>` command it dispatches to: the written head is
    /// the bare tail, the resolved / settled name is the mathfunc command, and
    /// the span covers just the function token so a rename rewrites the tail.
    #[test]
    fn expr_function_call_records_a_mathfunc_invocation() {
        let src = "proc ::tcl::mathfunc::myfunc {x} { return $x }\nexpr {myfunc($a) + 1}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| {
                i.name == "myfunc"
                    && i.resolved_qualified_name.as_deref() == Some("::tcl::mathfunc::myfunc")
            })
            .expect("mathfunc invocation recorded");
        assert_eq!(
            &src[inv.range.start() as usize..inv.range.end() as usize],
            "myfunc",
            "span should cover the function-name token only",
        );
        assert_eq!(inv.argc, Some(1), "one argument expression");
        assert!(
            inv.resolution_candidates
                .iter()
                .any(|c| c == "::tcl::mathfunc::myfunc"),
            "settled candidates should include the mathfunc command: {:?}",
            inv.resolution_candidates,
        );
    }

    /// A math function used before its introducing release is W002 (the
    /// `::tcl::mathfunc::<name>` command does not exist in the older core):
    /// `min` is 8.5+, the `is*` classification family is 9.0+.
    #[test]
    fn expr_function_before_its_release_is_disabled_in_dialect() {
        let flags = |src: &str, dialect: &str| {
            let mut a = Analyser::new();
            a.analyse(src, dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W002)
        };
        assert!(flags("expr {min(1, 2)}\n", "tcl8.4"), "min() is 8.5+");
        assert!(
            !flags("expr {min(1, 2)}\n", "tcl8.6"),
            "min() exists in 8.6"
        );
        assert!(flags("expr {isinf(1.0)}\n", "tcl8.6"), "isinf() is 9.0+");
        assert!(
            !flags("expr {isinf(1.0)}\n", "tcl9.0"),
            "isinf() exists in 9.0"
        );
        // An 8.4-era function is fine everywhere.
        assert!(!flags("expr {abs(-1)}\n", "tcl8.4"), "abs() is 8.4");
    }

    /// A nested function call (`sqrt(abs($x))`) records both the outer and the
    /// inner function as mathfunc invocations.
    #[test]
    fn expr_nested_function_calls_are_both_recorded() {
        let src = "expr {sqrt(abs($x))}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        for f in ["sqrt", "abs"] {
            assert!(
                r.command_invocations.iter().any(|i| {
                    i.name == f
                        && i.resolved_qualified_name.as_deref()
                            == Some(&format!("::tcl::mathfunc::{f}"))
                }),
                "{f} should be recorded: {:?}",
                r.command_invocations
                    .iter()
                    .map(|i| &i.name)
                    .collect::<Vec<_>>(),
            );
        }
    }

    /// An unrelated ordinary `proc sin` sharing a math function's bare tail
    /// name must never hijack `expr {sin(...)}` — the two live in entirely
    /// separate command tables in real Tcl (confirmed by the VM's own
    /// `tcl::mathfunc::*` dispatch, which never consults an ordinary
    /// top-level command). Before the two-candidate resettlement rule
    /// below, `finalise_invocation_resolutions`'s generic one-hop
    /// `{ns}::{name}` suffix-strip misparsed the mathfunc-qualified
    /// `::tcl::mathfunc::sin` as if `::tcl::mathfunc` were the *calling*
    /// namespace, and — once `sin` was `known` only as the unrelated
    /// global proc — silently rewrote the resolved name to `::sin`.
    #[test]
    fn expr_function_call_ignores_unrelated_same_named_global_proc() {
        let src = "proc sin {x} { return bogus }\nset y [expr {sin(1.0)}]\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "sin" && i.argc == Some(1))
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::sin"),
            "the unrelated proc sin must not hijack expr's sin(...): {inv:?}",
        );
    }

    /// TIP 232: a namespace-local `proc ::ns::tcl::mathfunc::f` shadows the
    /// global `::tcl::mathfunc::f` for a call made from inside `::ns` —
    /// confirmed real Tcl behaviour by the VM's
    /// `namespace_local_mathfunc_shadows_global_in_expr`
    /// (`tcl-vm/tests/tricky_resolution_e2e.rs`). The analyser must settle
    /// the call to the local override, not the fixed global form.
    #[test]
    fn expr_function_call_resolves_namespace_local_mathfunc_override() {
        let src = "namespace eval ::nsa::tcl::mathfunc {}\n\
                    proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "pf")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::nsa::tcl::mathfunc::pf"),
            "pf(1) inside ::nsa must resolve to the local override: {inv:?}",
        );
        assert_eq!(
            inv.resolution_candidates,
            vec!["::nsa::tcl::mathfunc::pf", "::tcl::mathfunc::pf"],
            "local-first, then global, matching the VM's own search order",
        );
    }

    /// The companion of the override case above: no local override exists at
    /// `::nsa::tcl::mathfunc::pf`, only a *global* user-defined one — the
    /// call still finds it via the two-candidate rule's fallback step.
    #[test]
    fn expr_function_call_falls_back_to_global_user_override_from_a_namespace() {
        let src = "proc ::tcl::mathfunc::pf {x} { return 10 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "pf")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::pf"),
            "no local override at ::nsa -- must fall back to the global proc: {inv:?}",
        );
    }

    /// The everyday case on the namespace-aware resolution path: a built-in
    /// (`sin`), called from inside a namespace with no override anywhere,
    /// must still settle to the global built-in slot — the
    /// collision/shadowing rules above must not disturb the common
    /// no-namespace, no-override call the diagnostic layer covers.
    #[test]
    fn expr_function_call_resolves_builtin_from_inside_a_namespace() {
        let src = "namespace eval ::nsa {\n    proc caller {} { return [expr {sin(1.0)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "sin")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::sin"),
            "a built-in with no override anywhere must settle globally: {inv:?}",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "must still draw no W123: {:?}",
            r.diagnostics,
        );
    }

    /// TIP 232 math functions are ordinary commands, so `namespace path`
    /// applies to their resolution exactly as it does to any other command —
    /// confirmed against the VM's own `resolve_command_fqn`, which routes
    /// every lookup (mathfunc calls included) through the same `ns_paths`-
    /// aware resolver. Neither the caller's own namespace nor the global
    /// slot defines `triple`; only the `namespace path` entry does.
    #[test]
    fn expr_function_call_honours_namespace_path() {
        let src = "namespace eval ::libns::tcl::mathfunc {}\n\
                    proc ::libns::tcl::mathfunc::triple {x} { return [expr {$x * 3}] }\n\
                    namespace eval ::consumer {\n    \
                        namespace path ::libns\n    \
                        proc caller {} { return [expr {triple(2)}] }\n\
                    }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "triple")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::libns::tcl::mathfunc::triple"),
            "must settle via the namespace path entry: {inv:?}",
        );
        assert_eq!(
            inv.resolution_candidates,
            vec![
                "::consumer::tcl::mathfunc::triple",
                "::libns::tcl::mathfunc::triple",
                "::tcl::mathfunc::triple",
            ],
            "current namespace, then the path entry, then global",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "must draw no W123: {:?}",
            r.diagnostics,
        );
    }

    /// [`Analyser::analyse_list_quoted_body`]'s *variable-reference*
    /// recording, isolated at the analyser tier.
    ///
    /// A gate that treated any non-`Str` body token as an opaque barrier
    /// would leave a script argument **built** with `list` — Tk's own
    /// `library/tk.tcl` writes `namespace eval :: [list source [file join
    /// $::tk_library $file.tcl]]` — without `record_arg_var_reads` running
    /// inside it, so the enclosing proc's parameter would look unread from its
    /// own scope.  This pins the analyser primitive that find-references and
    /// semantic tokens are built on.
    ///
    /// Oracle (tclsh 8.6.16 and 9.0.4): the parameter deterministically
    /// drives which file is sourced, so the read is real.
    #[test]
    fn a_list_built_body_records_its_parameter_reads_923_idx102() {
        // The read lands on the *scope tree*'s copy of the parameter — the
        // binding the LSP's variable providers resolve through.
        fn find<'a>(
            scope: &'a super::super::types::Scope,
            name: &str,
        ) -> Option<&'a super::super::types::VarDef> {
            scope
                .variables
                .get(name)
                .or_else(|| scope.children.iter().find_map(|c| find(c, name)))
        }
        let src = "proc ::app::SourceLibFile {file} {\n    \
                   namespace eval :: [list source [file join $::app::lib_dir $file.tcl]]\n}\n";
        let r = Analyser::new().analyse(src, "tcl8.6").clone();
        let param = find(&r.global_scope, "file")
            .expect("the `file` parameter must be recorded in the proc scope");
        // The recorded span covers the whole `$file` substitution, `$` included.
        let read_start = u32::try_from(src.find("$file.tcl").expect("the read is in the source"))
            .expect("offset fits u32");
        assert!(
            param.references.iter().any(|s| s.start() == read_start),
            "the `$file` read inside the list-built body must be recorded on the \
             parameter (expected a reference at {read_start}): {:?}",
            param.references,
        );
    }

    /// TN control for the above — a body that is genuinely dynamic
    /// (`[gen]`, not a statically known `list`-built command) stays the
    /// opaque barrier it always was, so nothing inside it is attributed.
    #[test]
    fn a_dynamic_body_records_no_parameter_reads_923_idx102() {
        let src = "proc ::app::Run {file} {\n    namespace eval :: [gen $file]\n}\n";
        let r = Analyser::new().analyse(src, "tcl8.6").clone();
        // The `$file` inside `[gen $file]` is a real argument read of the
        // enclosing command substitution and is recorded once; what must not
        // happen is the `[gen …]` body being *walked* as a script.
        assert!(
            !r.command_invocations
                .iter()
                .any(|i| i.name == "source" || i.name == "namespace eval"),
            "a dynamic body must not be walked as a script: {:?}",
            r.command_invocations
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
        );
    }
}

#[cfg(test)]
mod original_conditional_variable_read_tests {
    use crate::analyser::Analyser;

    #[test]
    fn original_conditional_variable_reads_keep_distinct_absolute_opaque_operands() {
        // Implementation contract: naming.variable.conditional-registry-receiver-geometry
        // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
        let source = r"namespace eval n\uD800 {}
namespace exists n\uD800
namespace eval N {}
set ::N::v\uD800 1
set ::N::v\uD801 2
info exists ::N::v\uD800
info exists ::N::v\uD801";
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let mut analysis = Analyser::new().analyse(source, profile);
            let image = tcl_lexer::SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            analysis.global_scope.variables.clear();
            analysis.command_invocations.clear();
            let mut symbols = Vec::new();
            for (spelling, bytes) in [
                (r"::N::v\uD800", b"::N::v\xed\xa0\x80".as_slice()),
                (r"::N::v\uD801", b"::N::v\xed\xa0\x81".as_slice()),
            ] {
                let offset = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
                let reference = analysis
                    .original_variable_symbol_in_source(&image, config, offset)
                    .unwrap_or_else(|| {
                        panic!("{profile}: original info-exists receiver {spelling}")
                    });
                assert!(!reference.is_declaration());
                assert_eq!(reference.original_name_input().bytes(), bytes);
                let declaration = analysis
                    .original_variable_symbols
                    .iter()
                    .find(|row| row.is_declaration() && row.symbol() == reference.symbol())
                    .unwrap_or_else(|| panic!("{profile}: corresponding opaque declaration"));
                assert_eq!(declaration.original_name_input().bytes(), bytes);
                symbols.push(reference.symbol().clone());
                assert!(
                    analysis
                        .original_variable_symbol_in_source(
                            &tcl_lexer::SourceImage::document(
                                &source.replace("namespace eval N", "namespace eval M")
                            ),
                            config,
                            offset,
                        )
                        .is_none(),
                    "{profile}: edited owner source is unavailable"
                );
            }
            assert_ne!(
                symbols[0], symbols[1],
                "{profile}: opaque keys stay distinct"
            );
        }
    }

    #[test]
    fn original_conditional_variable_reads_preserve_shadow_and_unknown_scope_barriers() {
        // Implementation contract: naming.variable.conditional-registry-receiver-geometry
        // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
        for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
            for (source, operand) in [
                (
                    "proc info {args} {return CUSTOM}; info exists ::target",
                    "::target",
                ),
                (
                    "unknown_command; namespace eval N {info exists bare}",
                    "bare",
                ),
            ] {
                let analysis = Analyser::new().analyse(source, profile);
                let image = tcl_lexer::SourceImage::document(source);
                let config = analysis.body_lexer_config.unwrap();
                let offset = u32::try_from(source.rfind(operand).unwrap()).unwrap();
                assert!(
                    analysis
                        .original_variable_symbol_in_source(&image, config, offset)
                        .is_none(),
                    "{profile}: conditional advice cannot select {operand}"
                );
            }
        }
    }
}

#[cfg(test)]
mod original_lexical_source_inventory_tests {
    use crate::analyser::Analyser;

    #[test]
    fn original_lexical_inventory_retains_brackets_without_parent_execution() {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        for source in [
            "when HTTP_REQUEST {ILX::call [ILX::init p e] m}",
            "unknown [ILX::init p e]",
            "unknown \"prefix[ILX::init p e]suffix\"",
            "unknown $array([ILX::init p e])",
            "unknown [unknown [ILX::init p e]]",
        ] {
            let analysis = Analyser::new().analyse(source, "f5-irules");
            let image = tcl_lexer::SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("ILX::init").unwrap()).unwrap();
            let occurrence = analysis
                .original_vendor_source_names()
                .find(|occurrence| {
                    occurrence.site().offset == offset
                        && occurrence.original_words().first()
                            == Some(occurrence.name_input().original_word())
                })
                .unwrap_or_else(|| panic!("missing genuine lexical constructor: {source}"));
            assert_eq!(occurrence.original_words().len(), 3, "{source}");
            let last = occurrence.original_words().last().unwrap();
            assert_eq!(
                source.get(offset as usize..last.span().end() as usize),
                Some("ILX::init p e")
            );
            assert!(
                occurrence
                    .original_words()
                    .iter()
                    .all(|word| word.image() == &image && word.config() == config)
            );
            assert_eq!(
                occurrence
                    .name_input()
                    .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead),
                Some(&b"ILX::init"[..])
            );
            assert!(
                analysis
                    .original_vendor_source_name_in_source(
                        &image,
                        config,
                        occurrence.name_input().span()
                    )
                    .is_some()
            );
            assert!(
                analysis
                    .original_vendor_source_name_in_source(
                        &tcl_lexer::SourceImage::document(&format!("{source}\n# changed")),
                        config,
                        occurrence.name_input().span()
                    )
                    .is_none()
            );
            assert!(
                analysis.original_completed_command_world().is_none(),
                "lexical children cannot supply a completed world: {source}"
            );
            assert_eq!(
                analysis.original_procedure_declarations().count(),
                0,
                "hosted syntax cannot publish native procedures"
            );
        }
    }

    #[test]
    fn original_lexical_inventory_keeps_inert_braces_and_escapes_opaque() {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        for source in [
            "unknown {[ILX::init p e]}",
            r"unknown \[ILX::init p e\]",
            "unknown [set text {[ILX::init p e]}]",
        ] {
            let analysis = Analyser::new().analyse(source, "f5-irules");
            let offset = u32::try_from(source.find("ILX::init").unwrap()).unwrap();
            assert!(
                !analysis
                    .original_vendor_source_names()
                    .any(|occurrence| occurrence.site().offset == offset),
                "inert source cannot acquire a command vector: {source}"
            );
            assert!(analysis.original_completed_command_world().is_none());
        }
    }
}

#[cfg(test)]
mod original_script_body_inventory_tests {
    use crate::analyser::Analyser;
    use tcl_lexer::SourceImage;

    #[test]
    fn original_script_inventory_uses_whole_authored_body_and_case_regions() {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for source in [
            "proc f {} { ILX::call $h m }",
            "eval { ILX::call $h m }",
            "when X { if {1} { ILX::call $h m } }",
            "when X { foreach i {1 2} { ILX::call $h m } }",
            "when X { catch { ILX::call $h m } }",
            "when X { while {0} { ILX::call $h m } }",
            "when X { switch [HTTP::uri] { /api { ILX::call $h m } } }",
            "when X { switch [HTTP::uri] /api { ILX::call $h m } }",
            "proc f {} { set h [ILX::init p e]; ILX::call $h m }",
        ] {
            let analysis = Analyser::new().analyse(source, "f5-irules");
            let image = SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("ILX::call").unwrap()).unwrap();
            let occurrence = analysis
                .original_vendor_source_names()
                .find(|occurrence| {
                    occurrence.site().offset == offset
                        && occurrence.original_words().first()
                            == Some(occurrence.name_input().original_word())
                })
                .unwrap_or_else(|| panic!("missing genuine script-body call: {source}"));
            assert!(occurrence.body_origin().is_some(), "{source}");
            assert_eq!(occurrence.original_words().len(), 3, "{source}");
            assert!(
                occurrence
                    .original_words()
                    .iter()
                    .all(|word| word.image() == &image && word.config() == config)
            );
            let (metadata, _) =
                crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                    source, &analysis, offset,
                )
                .unwrap_or_else(|| panic!("missing readonly source schema: {source}"));
            assert_eq!(metadata.shape().command(), "ILX::call");
            assert!(metadata.matches_source(&image, config));
            assert!(
                analysis.original_completed_command_world().is_none(),
                "stored body syntax cannot donate a completed world: {source}"
            );
            assert_eq!(analysis.original_procedure_declarations().count(), 0);
            assert!(
                crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                    &format!("{source}\n# changed"),
                    &analysis,
                    offset,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_script_inventory_keeps_inert_cooked_and_blocked_bodies_opaque() {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        // The moved-handler control has an independently checked positive
        // parent and child schema in the very same hosted authoring context.
        let baseline = "if 1 { puts baseline }";
        let baseline_analysis = Analyser::new().analyse(baseline, "f5-tmsh");
        let baseline_offset = u32::try_from(baseline.find("puts").unwrap()).unwrap();
        let baseline_child = baseline_analysis
            .original_vendor_source_names()
            .find(|occurrence| {
                occurrence.site().offset == baseline_offset
                    && occurrence.original_words().first()
                        == Some(occurrence.name_input().original_word())
            })
            .expect("tmsh if body must retain its genuine common child");
        assert!(baseline_child.body_origin().is_some());
        let (baseline_metadata, baseline_segment) =
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                baseline,
                &baseline_analysis,
                baseline_offset,
            )
            .expect("tmsh puts baseline must select its readonly source schema");
        assert_eq!(baseline_metadata.shape().command(), "puts");
        assert_eq!(baseline_segment.argv[0].span.start(), baseline_offset);
        for source in [
            "set text { ILX::call $h m }",
            "unknown { ILX::call $h m }",
            r#"if 1 "ILX::call\ \$h\ m""#,
            "proc if {args} {}; if 1 { ILX::call $h m }",
            "rename if saved_if; if 1 { puts blocked }",
            "eval { ILX::call } { $h m }",
        ] {
            // Direct rename is not in the TMM authoring surface. A known
            // moved-handler barrier must select its actual hosted context.
            let dialect = if source.starts_with("rename if") {
                "f5-tmsh"
            } else {
                "f5-irules"
            };
            let analysis = Analyser::new().analyse(source, dialect);
            if source.starts_with("rename if") {
                let context = analysis.resolved_input.as_ref().unwrap().context_registry();
                assert!(
                    context
                        .context()
                        .resolve_spec(context.commands(), "rename")
                        .is_some()
                );
            }
            let child = if source.starts_with("rename if") {
                "puts"
            } else {
                "ILX::call"
            };
            let offset = u32::try_from(source.find(child).unwrap()).unwrap();
            let occurrences = analysis
                .original_vendor_source_names()
                .filter(|occurrence| occurrence.site().offset == offset)
                .collect::<Vec<_>>();
            assert!(
                occurrences
                    .iter()
                    .all(|occurrence| occurrence.body_origin().is_none()),
                "inert, cooked, blocked or concatenated source cannot donate a body-role receipt: {source}"
            );
            assert!(
                crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                    source, &analysis, offset,
                )
                .is_none_or(|(_, actual)| actual
                    .argv
                    .first()
                    .is_none_or(|head| head.span.start() != offset)),
                "an inert cursor or lexical vector cannot supply child source roles: {source}"
            );
            if !source.starts_with("proc if") && !source.starts_with("rename if") {
                assert!(
                    occurrences.is_empty(),
                    "inert, cooked or concatenated source is not a script child: {source}"
                );
            }
        }
    }
}

#[cfg(test)]
mod original_nested_source_tests {
    use super::*;

    fn nested_heads(source: &str, dialect: &str) -> Vec<CollectedHead> {
        let analysis = Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let sm = SourceMap::new(source);
        let mut heads = Vec::new();
        let mut expressions = Vec::new();
        let last = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        for token in last
            .all_tokens
            .iter()
            .filter(|token| token.kind == TokenType::Cmd)
        {
            collect_substitution_heads(
                &sm,
                &analysis,
                *token,
                config,
                &mut heads,
                &mut expressions,
            );
        }
        heads
    }

    #[test]
    fn named_receiver_inventory_uses_original_ordered_source_carriers() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        for (source, marker, expected) in [
            (
                "oo::class create C {}; C create obj; obj ping",
                "obj ping",
                true,
            ),
            (
                "oo::class create C {}; C create obj; rename obj moved; moved ping",
                "moved ping",
                true,
            ),
            (
                "oo::class create C {}; C create obj; rename obj {}; obj ping",
                "obj ping",
                false,
            ),
            (
                "oo::class create C {}; C create obj; proc obj args {}; obj ping",
                "obj ping",
                false,
            ),
            (
                "oo::class create C {}; C create obj; rename C {}; obj ping",
                "obj ping",
                false,
            ),
        ] {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
            let recorded = analyser.var_command_sites.iter().any(|site| {
                site.cmd_span.start() == offset
                    && site.receiver == super::super::state::DispatchReceiver::InstanceCommand
            });
            assert_eq!(recorded, expected, "{source}");
            assert_eq!(
                crate::registry_invocation::source_structure::source_class_instance_words_at(
                    source, &analysis, offset,
                )
                .is_some(),
                expected,
                "{source}",
            );
            assert!(analysis.created_instance_commands.is_empty());
        }
    }

    #[test]
    fn direct_callbacks_retain_selected_schema_and_exact_prefix_producers() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for source in [
            "lsort -command {compare fixed} {b a}",
            "rename lsort sorter; sorter -command {compare fixed} {b a}",
            "interp alias {} sorter {} lsort -command {compare fixed}; sorter {b a}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let callbacks = analysis
                .command_invocations
                .iter()
                .filter(|invocation| {
                    invocation.name == "compare" && invocation.callback_arity.is_some()
                })
                .collect::<Vec<_>>();
            let [callback] = callbacks.as_slice() else {
                panic!("expected exactly one authentic callback: {source}");
            };
            let prefix = callback.original_callback_prefix.as_ref().unwrap();
            assert_eq!(prefix.name_input().bytes(), b"compare");
            assert_eq!(prefix.baked_argument_count(), 1);
            assert_eq!(callback.callback_arity, prefix.appended_arity());
            assert_eq!(callback.callback_baked_args, prefix.baked_argument_count());
            assert_eq!(
                callback.original_name_input.as_ref(),
                Some(prefix.name_input())
            );
            assert!(callback.argc.is_none() && !callback.rename_safe);
        }
        for source in [
            "proc lsort args {}; lsort -command {compare fixed} {b a}",
            "rename lsort {}; lsort -command {compare fixed} {b a}",
            "lsort -command $computed {b a}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(
                !analysis
                    .command_invocations
                    .iter()
                    .any(|invocation| invocation.name == "compare"
                        && invocation.callback_arity.is_some()),
                "{source}"
            );
        }
    }

    #[test]
    fn nested_source_bodies_follow_original_moves_alias_prefixes_and_shadow_barriers() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for source in [
            "rename switch choose; set result [choose key {key {puts nested}}]",
            "interp alias {} choose {} switch --; set result [choose key {key {puts nested}}]",
        ] {
            let heads = nested_heads(source, "tcl8.6");
            assert!(heads.iter().any(|head| head.name == "puts"), "{source}");
            assert!(
                !heads.iter().any(|head| head.name == "key"),
                "a case pattern is data"
            );
        }
        let source = "proc switch args {}; set result [switch key {key {puts inert}}]";
        let heads = nested_heads(source, "tcl8.6");
        assert!(heads.iter().any(|head| head.name == "switch"));
        assert!(!heads.iter().any(|head| head.name == "puts"));
    }

    #[test]
    fn nested_source_bodies_use_the_actual_dialect_availability() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "set result [dict for {key value} {} {puts nested}]";
        assert!(
            nested_heads(source, "tcl8.6")
                .iter()
                .any(|head| head.name == "puts")
        );
        assert!(
            !nested_heads(source, "tcl8.4")
                .iter()
                .any(|head| head.name == "puts")
        );
    }

    #[test]
    fn nested_source_callbacks_keep_the_original_input_baked_count_and_purpose() {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        let source = "proc compare args {}; set result [lsort -command {compare fixed} {a b}]";
        let heads = nested_heads(source, "tcl8.6");
        let head = heads.iter().find(|head| head.name == "compare").unwrap();
        let callback = head.callback.as_ref().unwrap();
        assert_eq!(callback.name_input().bytes(), b"compare");
        assert_eq!(callback.baked_argument_count(), 1);
        assert_eq!(
            callback.appended_arity(),
            Some(tcl_registry::AppendedArity::Exactly(2))
        );
        assert_eq!(source.get(head.span.as_range()), Some("compare"));
        assert!(head.argc.is_none());
    }

    fn reference_only_script(
        _arguments: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }

    #[test]
    fn nested_source_inventory_keeps_reference_only_scripts_out_of_dispatch() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        let source = "set result [puts {format nested}]";
        for reference_only in [true, false] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let generation = tcl_registry::model::context_for_profile(profile);
            let mut registry = tcl_registry::CommandRegistry::build_default();
            registry.insert(tcl_registry::CommandSpec {
                name: "puts",
                arity: tcl_registry::Arity::exact(1),
                arg_roles: &[(0, tcl_registry::ArgRole::Body)],
                arg_role_layout_resolver: None,
                arg_role_count_resolver: None,
                arg_role_resolver: None,
                arg_role_resolver_roles: &[],
                script_timing_resolver: reference_only.then_some(reference_only_script),
                ..generation
                    .context()
                    .resolve_spec(generation.commands(), "puts")
                    .unwrap()
                    .clone()
            });
            let context =
                std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(registry)));
            let selected = context
                .context()
                .resolve_spec(context.commands(), "puts")
                .unwrap();
            assert_eq!(selected.arg_roles, &[(0, tcl_registry::ArgRole::Body)]);
            assert_eq!(selected.script_timing_resolver.is_some(), reference_only);
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input =
                super::super::input::ResolvedAnalysisInput::new(profile, profile, context, config);
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let sm = SourceMap::new(source);
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .pop()
                    .unwrap();
            let token = *segment
                .all_tokens
                .iter()
                .find(|token| token.kind == TokenType::Cmd)
                .unwrap();
            let mut heads = Vec::new();
            let mut expressions = Vec::new();
            collect_substitution_heads(&sm, &analysis, token, config, &mut heads, &mut expressions);
            assert!(
                heads.iter().any(|head| head.name == "format"),
                "readonly syntax stays visible"
            );
            let mut potential = Vec::new();
            collect_substitution_segments(&sm, &analysis, token, config, &mut potential);
            assert_eq!(
                potential
                    .iter()
                    .any(|segment| segment.texts.first().is_some_and(|head| head == "format")),
                !reference_only
            );
        }
    }

    #[test]
    fn source_user_class_setters_join_original_moves_aliases_and_whole_substitutions() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "oo::class create C {}; rename C Held; set object [Held \"new\"]",
            "oo::class create C {}; rename C Held; interp alias {} make {} Held new; set object [make]",
        ] {
            let mut analyser = Analyser::new();
            let result = analyser.analyse(source, "tcl8.6");
            assert_eq!(
                result.instance_classes.get("object").map(String::as_str),
                Some("::C"),
                "{source}"
            );
            let offset = u32::try_from(source.find("set object").unwrap()).unwrap();
            let construction =
                crate::registry_invocation::source_structure::source_handle_construction_at(
                    source, &result, offset,
                )
                .unwrap();
            let call = crate::registry_invocation::source_structure::source_constructor_call_at(
                source,
                &result,
                construction.construction().words[0].span().start(),
            )
            .unwrap();
            assert!(call.constructor_shape(&result).is_some());
            assert_eq!(
                call.class_declaration()
                    .source_class(&result)
                    .unwrap()
                    .metadata()
                    .qualified_name,
                "::C"
            );
        }
        for source in [
            "oo::class create C {}; proc set args {}; set object [C new]",
            "oo::class create C {}; set object [C new; format changed]",
            "oo::class create C {}; set object \"prefix[C new]\"",
            "oo::class create C {}; set object [C new] surplus",
            "oo::class create C {}; proc C args {}; set object [C new]",
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            assert!(!result.instance_classes.contains_key("object"), "{source}");
        }
    }

    #[test]
    fn pending_constructor_heads_preserve_real_variable_and_selector_geometry() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        for (source, expected) in [
            (
                r"set object [$class create name]",
                Some(("class", "create", "$class")),
            ),
            (
                r"set object [${class with space} n\ew]",
                Some(("class with space", "new", "${class with space}")),
            ),
            (r"set object [${class}Suffix create name]", None),
            (r"set object [$class(index) create name]", None),
            (r"set object [$class $selector name]", None),
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            let construction =
                crate::registry_invocation::source_structure::source_handle_construction_at(
                    source, &result, 0,
                )
                .unwrap();
            let pending = class_var_head_constructor_subst(&construction);
            assert_eq!(
                pending.as_ref().map(|(class, method, span)| (
                    class.as_str(),
                    method.as_str(),
                    &source[span.as_range()]
                )),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn source_named_constructors_do_not_publish_permanent_speculative_commands() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create C {}; rename C Held; interp alias {} make {} Held create; make object";
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            result.instance_classes.get("object").map(String::as_str),
            Some("::C")
        );
        assert!(result.created_instance_commands.is_empty());
        for source in [
            "Unknown create object",
            "oo::class create C {}; proc C args {}; C create object",
            "oo::class create C {}; rename C {}; C create object",
            "oo::class create C {self method create args {return ordinary}}; C create object",
        ] {
            let result = Analyser::new().analyse(source, "tcl8.6");
            assert!(!result.instance_classes.contains_key("object"), "{source}");
            assert!(
                !result.created_instance_commands.contains("object"),
                "{source}"
            );
        }
    }
    #[test]
    fn source_pending_constructor_values_do_not_select_native_reporting_class_labels() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create C {}; set class C; set object [$class new]";
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let mut analyser = Analyser::new();
            analyser.result = analyser.analyse(source, dialect);
            assert!(
                !analyser
                    .resolved_analysis_input()
                    .has_logical_source_name_context(),
                "{dialect}"
            );
            assert!(
                analyser.resolve_user_class_at("C", 0).is_none(),
                "{dialect}"
            );
            assert!(
                !analyser.result.instance_classes.contains_key("object"),
                "{dialect}"
            );
        }
        let mut logical = Analyser::new();
        logical.result = logical.analyse("oo::class create C {}", "tcl");
        assert!(
            logical
                .resolved_analysis_input()
                .has_logical_source_name_context()
        );
        assert_eq!(
            logical.resolve_user_class_at("C", 0).as_deref(),
            Some("::C")
        );
        logical.result.resolved_input = None;
        logical.result.lexical_declaration_advice = true;
        assert!(logical.resolve_user_class_at("C", 0).is_none());
        assert!(logical.resolve_user_class_in("C", &[]).is_none());
    }
    #[test]
    fn logical_reported_class_lookup_respects_lexical_candidates_without_native_identity() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source = "oo::class create Global {}; namespace eval ns {oo::class create Local {}; namespace eval inner {set marker 1}}";
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, "tcl");
        let local = u32::try_from(source.find("oo::class create Local").unwrap()).unwrap();
        let inner = u32::try_from(source.find("set marker").unwrap()).unwrap();
        assert_eq!(
            analyser.resolve_user_class_at("Local", local).as_deref(),
            Some("::ns::Local")
        );
        assert_eq!(
            analyser.resolve_user_class_at("Global", inner).as_deref(),
            Some("::Global")
        );
        assert!(
            analyser.resolve_user_class_at("Local", inner).is_none(),
            "no ancestor or unique-tail fallback"
        );
        analyser.result.resolved_input = None;
        analyser.result.lexical_declaration_advice = true;
        assert!(analyser.resolve_user_class_at("Global", inner).is_none());
    }
    #[test]
    fn analyser_hook_metadata_uses_retained_context_without_stashed_store_donation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let generation = tcl_registry::model::context_for_profile(profile);
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            analyser_hook: None,
            ..generation
                .context()
                .resolve_spec(generation.commands(), "set")
                .unwrap()
                .clone()
        });
        let context =
            std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(registry)));
        let input = super::super::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.context = None;
        analyser.registry = Some(std::sync::Arc::new(
            tcl_registry::CommandRegistry::build_default(),
        ));
        assert!(
            analyser
                .resolve_analyser_hook_call("set", &["name".into(), "value".into()])
                .is_none()
        );
        let old_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let input = super::super::ResolvedAnalysisInput::new(
            old_profile,
            old_profile,
            tcl_registry::model::context_for_profile(old_profile),
            LexerConfig::for_file_grammar(old_profile.grammar),
        );
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.context = None;
        analyser.registry = Some(std::sync::Arc::new(
            tcl_registry::CommandRegistry::build_default(),
        ));
        assert!(!analyser.head_is_self_dispatch_keyword("my"));
    }
}
