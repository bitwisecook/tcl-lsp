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

//! The command loop: a framed script, one command and the substitution and dispatch of its words.

use super::super::limits::NATIVE_EVAL_DEPTH_LIMIT;
use super::super::{
    native_compilation, native_operation_currency, new_string, obj_bytes, release_all,
    trace_native_depth_refusal, CmdFrame, Code, FrameKind, Interp, OriginalJimEvaluationFrame,
};
use super::{count_newlines, line_of, scan_list_offsets};
use crate::obj::TclObj;
use crate::parse::{WordBody, WordPart};
use crate::{obj, parse};
use tcl_runtime_api::jim_error_stack::JimScriptLocation;
use tcl_runtime_api::CommandEvaluation;

impl Interp {
    /// Evaluate `src` as the body of its own `info frame` level (`frame` is
    /// pushed for the duration). Used by proc calls, `eval`/`uplevel`, and
    /// `source`.
    pub(in super::super) fn eval_framed(&mut self, src: &[u8], mut frame: CmdFrame) -> Code {
        frame.proc_line_base = frame.line_base;
        self.eval_script(src, Some(frame))
    }

    /// [`Self::eval_framed`] before its outermost publication tail. This is
    /// reserved for a source boundary, which must settle `return` first and may
    /// snapshot the resulting completion before publication.
    pub(super) fn eval_framed_unpublished(&mut self, src: &[u8], mut frame: CmdFrame) -> Code {
        // A freshly pushed frame is its own `codePtr->source`, so `errorLine` is
        // measured from this body's base (an inline `catch`/`if` body, by
        // contrast, shares the enclosing frame via `eval_shared_located_body` and
        // keeps the proc's `proc_line_base`).
        frame.proc_line_base = frame.line_base;
        self.eval_script_mode_unpublished(src, Some(frame), false)
    }

    /// The shared command loop. If `owned` is `Some`, it is pushed as this
    /// script's `CmdFrame` and updated to each command as the loop steps through
    /// it (so `info frame` sees the live command/line); an unframed call (`None`,
    /// i.e. command substitution) leaves the enclosing frame untouched.
    fn eval_script(&mut self, src: &[u8], owned: Option<CmdFrame>) -> Code {
        self.eval_script_mode(src, owned, false)
    }

    /// [`eval_script`](Self::eval_script) with an explicit `advance_shared` mode:
    /// when `owned` is `None` but `advance_shared` is set, the enclosing frame is
    /// shared (not pushed — no new level) yet its `line`/`cmd` still advance with
    /// each command. This is the command-substitution case (see
    /// [`eval_command_subst`](Self::eval_command_subst)); the caller sets up and
    /// restores the shared frame's `line_base`.
    pub(in super::super) fn eval_script_mode(
        &mut self,
        src: &[u8],
        owned: Option<CmdFrame>,
        advance_shared: bool,
    ) -> Code {
        self.observe_codegen_depth("script-before", 0);
        let code = self.eval_script_mode_unpublished(src, owned, advance_shared);
        self.observe_codegen_depth("script-returned", 0);
        self.finish_outermost_eval(code);
        code
    }

    /// The shared command loop through depth/frame unwind, stopping before the
    /// outermost publication tail. Public result adapters use this seam to
    /// snapshot live return options; ordinary evaluators immediately pass the
    /// code to [`Self::finish_outermost_eval`].
    pub(in super::super) fn eval_script_mode_unpublished(
        &mut self,
        src: &[u8],
        owned: Option<CmdFrame>,
        advance_shared: bool,
    ) -> Code {
        if !self.evaluation_is_live() {
            return Code::Error;
        }
        // The C object callback has no interpreter argument. Activate the
        // selected engine policy before evaluating; its precision remains
        // shared by interpreters of that engine on this thread.
        if let Some(policy) = self.native_invocation_dialect().double_string_policy() {
            obj::install_double_string_policy(policy);
        }
        let dialect = self.native_invocation_dialect();
        if let Some(release) = dialect.tcl_version {
            tcl_syntax::number::set_runtime_release(release);
        }
        parse::install_native_list_policy(
            dialect.lexer_grammar.list_parse,
            dialect.lexer_grammar.escapes,
        );
        // Native-stack safety net — see `NATIVE_EVAL_DEPTH_LIMIT`'s doc
        // comment. Checked before incrementing / doing any other setup, so
        // bailing out here needs no unwind: `owned` (not yet pushed) simply
        // drops normally.
        let next_proc_body = self.next_proc_body.replace(false);
        let top_level_file = self.next_top_level_file.replace(false) && owned.is_some();
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            #[cfg(debug_assertions)]
            trace_native_depth_refusal(
                "script-body",
                self.eval_depth.get(),
                self.native_dispatch_depth.get(),
            );
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        if self.eval_depth.get() == 0 {
            self.reset_native_compilation_admission();
        }
        if let Err(failure) = self.enter_native_script(src) {
            return self.admit_native_compilation_error(&failure, None);
        }
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
        ) {
            self.leave_native_script();
            return code;
        }
        if self.uses_jim_error_stack() {
            self.jim_error_stack.borrow_mut().mark_reset();
        }
        let pushed = owned.is_some();
        let evaluation = self.next_evaluation.take().unwrap_or(if pushed {
            CommandEvaluation::Compiled
        } else {
            self.evaluation.get()
        });
        // A release that compiles a script whole refuses it before any of it
        // runs; a procedure body is compiled as its procedure is called, and
        // stdin a chunk at a time.
        if pushed && evaluation == CommandEvaluation::Compiled && !next_proc_body {
            if let Some(failure) = self.compile_failure_84(src, false, self.current_ns.get()) {
                self.leave_native_script();
                return self.raise_compile_failure(&failure, b"");
            }
        }
        self.eval_depth.set(self.eval_depth.get() + 1);
        let outer_evaluation = self.evaluation.replace(evaluation);
        let proc_body = pushed && next_proc_body;
        let outer_proc_body = if pushed {
            self.proc_body.replace(proc_body)
        } else {
            self.proc_body.get()
        };
        let owns_frame = pushed || advance_shared;
        if let Some(f) = owned {
            self.cmd_frames.borrow_mut().push(f);
        }
        let mut last = Code::Ok;
        let commands = if matches!(
            self.native_invocation_dialect()
                .native_source_string_protocol(),
            Some(tcl_syntax::native_string::NativeStringProtocol::C(_))
        ) {
            parse::parse_script_with_original_text(src, self.lexer_config())
        } else {
            parse::parse_script_with_config(src, self.lexer_config())
        };
        let failure = (self.native_invocation_dialect().script_parse_timing()
            == Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript))
        .then(|| {
            commands
                .iter()
                .find_map(|command| parse::first_parse_error(&command.words, self.lexer_config()))
        })
        .flatten();
        if let Some(message) = failure {
            last = self.error(message.as_bytes());
            if let Err(first) = self.capture_jim_script_parse_failure() {
                return self.refuse_native_execution(first);
            }
        }
        if commands.is_empty() {
            // A script with no commands (empty / whitespace / comments only)
            // evaluates to the empty result — `Tcl_EvalEx` resets the result at
            // entry, and with nothing to set it the result is empty. Without this
            // a stale prior result leaks through (e.g. an empty proc body, `eval
            // {}`, or an `lmap`/`foreach` body that produces nothing).
            self.set_result_bytes(b"");
        }
        for cmd in &commands {
            if last != Code::Ok {
                break;
            }
            last = self.eval_command(src, cmd, owns_frame);
            if self.host_refusal_pending() {
                last = Code::Error;
            }
            if last != Code::Ok {
                if top_level_file {
                    last = self.settle_top_level_command(src, cmd, last);
                }
                break; // error/return/break/continue propagate up
            }
        }
        if pushed {
            self.cmd_frames.borrow_mut().pop();
        }
        self.evaluation.set(outer_evaluation);
        self.proc_body.set(outer_proc_body);
        self.eval_depth.set(self.eval_depth.get() - 1);
        self.leave_native_script();
        last
    }

    /// Apply the one outermost-evaluation tail after any result/options
    /// projection has observed the live completion state.
    pub(in super::super) fn finish_outermost_eval(&mut self, code: Code) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.host_refusal_pending() || self.eval_depth.get() != 0 {
            return;
        }
        if code == Code::Error {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                if self.propagate_error_stage().is_err() {
                    return;
                }
            } else {
                // C8.4 keeps ERR_IN_PROGRESS and ERROR_CODE_SET until the next
                // Tcl_ResetResult; the next outermost evaluation resets them.
                self.publish_error_episode(!self.uses_c84_global_error_info());
            }
        }
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        // Between top-level commands, drain any queued background errors with the
        // current handler — the event loop's behaviour, so errors from one
        // command don't leak into a later command's intercepted handler.
        if !self.bg_queue.borrow().is_empty() {
            self.process_bg_errors();
        }
    }

    /// A `CmdFrame` for an `eval` body, inheriting the current frame's
    /// kind/proc/level/file context (the body runs in the enclosing CallFrame).
    /// `line_base` is the line of the `eval` command minus one — the body opens
    /// on that line, so in a sourced context its commands stay file-absolute
    /// (e.g. `eval` at file line 5 → its body at line 5). `uplevel`/`source`
    /// override fields ([`eval_uplevel`](Self::eval_uplevel)/
    /// [`eval_sourced`](Self::eval_sourced)).
    pub(super) fn inherited_cmd_frame(&self) -> CmdFrame {
        let line_base = self.current_cmd_line().saturating_sub(1);
        let cmd_frames = self.cmd_frames.borrow();
        let top = cmd_frames.last();
        CmdFrame {
            kind: top.map_or(FrameKind::Eval, |f| f.kind),
            file: top.and_then(|f| f.file.clone()),
            proc: top.and_then(|f| f.proc.clone()),
            level: top.map_or(0, |f| f.level),
            // Inherit the enclosing frame's level-reachability (an inline body
            // of an `uplevel`-redirected script also omits `level`).
            omit_level: top.is_some_and(|f| f.omit_level),
            // An `eval`/body frame runs in the enclosing call frame — inherit its
            // identity so the `info frame` `level` reachability test sees it as
            // the same CallFrame.
            frame_index: top.map_or(0, |f| f.frame_index),
            line_base,
            proc_line_base: line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo: top.and_then(|f| f.oo.clone()),
            lambda: top.and_then(|f| f.lambda.clone()),
        }
    }

    /// Evaluate one parsed command, then — if it errored — append its
    /// `while executing` / `invoked from within` frame to the error trace
    /// (`TclLogCommandInfo`), using the command's source slice and line.
    pub(in super::super) fn eval_command(
        &mut self,
        src: &[u8],
        cmd: &parse::Command,
        owns_frame: bool,
    ) -> Code {
        // Update the current `info frame` level to this command — the innermost
        // command executing at this level. A `[cmd]` substitution (and an inline
        // control body) shares the enclosing frame and adds no level, so it
        // updates the `cmd` but keeps the **enclosing** command's `line` (the
        // line the substitution appears on); only the frame-owning script
        // advances the line as it steps through its own commands.
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            if owns_frame {
                // `line_base` shifts a source-defined proc's body lines to be
                // file-absolute; it is 0 elsewhere (body-relative).
                top.line = top.line_base + line_of(src, cmd.start);
            }
            top.cmd = src[cmd.start..cmd.end].to_vec();
            top.original_command = None;
        }
        if self.uses_jim_error_stack() {
            let script = self
                .cmd_frames
                .borrow()
                .last()
                .map(|frame| JimScriptLocation {
                    file: frame.file.as_deref().unwrap_or(b"").to_vec(),
                    line: frame.line_base + line_of(src, cmd.start),
                });
            self.jim_evaluation_frames
                .borrow_mut()
                .push(OriginalJimEvaluationFrame {
                    procedure_level: self.jim_procedure_level.get(),
                    command: None,
                    is_procedure: false,
                    script,
                    invocation: Vec::new(),
                });
        }
        let selected = self.native_command_selection(src, cmd);
        let code = self.eval_words(src, &cmd.words, selected);
        if code == Code::Error {
            self.log_command_info(src, cmd);
        }
        if self.uses_jim_error_stack() {
            self.jim_evaluation_frames.borrow_mut().pop();
        }
        code
    }

    /// Substitute each word of a command (with `{*}` expansion), then dispatch.
    fn eval_words(
        &mut self,
        src: &[u8],
        words: &[parse::Word],
        selected: Option<Box<native_compilation::SelectedInvocation>>,
    ) -> Code {
        // C parses a command WHOLE before it substitutes any of it
        // (`Tcl_EvalEx` → `Tcl_ParseCommand` → `TclEvalObjvInternal`), so a
        // parse failure in a later word — or inside a later word's `[…]` —
        // stops an earlier word's command substitution from ever running.
        // Measured on 8.6.16 and 9.0.4: `list [sfx inner] {a}b` raises `extra
        // characters after close-brace` with `sfx` never called. Walking the
        // words in order and raising at the first `WordPart::ParseError` (this
        // engine's carrier for those failures — the scanner stays infallible so
        // the LSP can keep tokenising) matches that: substituting word by word
        // without this check would run `sfx` before the later parse error is
        // seen.
        if let Some(msg) = parse::first_parse_error(words, self.lexer_config()) {
            return self.error(msg.as_bytes());
        }
        // The command's reported line + source file (for TIP 280 argument-line
        // tracking and the LABC literal-location table), and the first word's
        // offset (to measure each word's line within the command).
        let cmd_line = self.cmd_frames.borrow().last().map_or(0, |f| f.line);
        let file = self.cmd_frames.borrow().last().and_then(|f| f.file.clone());
        let w0 = words.first().map_or(0, |w| w.start);

        let mut argv: Vec<*mut TclObj> = Vec::new();
        // Per-**argv-element** file-absolute line (aligned with `argv`, so `{*}`
        // expansion stays correct), and the argv indices of literal arguments to
        // record in the LABC table (`eval`/`uplevel`/proc body source locations).
        let mut arg_lines: Vec<u32> = Vec::new();
        let mut labc: Vec<usize> = Vec::new();
        // The literal words of a command the release compiles. The parser
        // expands a literal `{*}` whose elements are all literal into literal
        // words; any other `{*}` leaves the command to the generic
        // invocation, as does a substituted command word.
        let mut literals: Vec<usize> = Vec::new();
        let mut compiled = self.evaluation.get() == CommandEvaluation::Compiled;

        for w in words {
            let word_line = cmd_line + count_newlines(&src[w0..w.start.min(src.len())]);
            let is_literal = matches!(w.body, parse::WordBody::Literal(_));
            self.begin_native_arguments();
            let substituted = self.substitute_word(src, w);
            self.end_native_arguments();
            let obj = match substituted {
                Ok(o) => o, // owned (+1)
                Err(code) => {
                    release_all(&argv);
                    return code;
                }
            };
            let obj = if w.kind == parse::WordKind::Quoted
                && self
                    .native_invocation_dialect()
                    .quoted_substitution_preserves_object()
                    == Some(false)
                && matches!(&w.body, WordBody::Parts(parts) if parts.len() == 1
                    && matches!(&parts[0], WordPart::Variable(_) | WordPart::Command(_) | WordPart::Expression(_)))
            {
                let copy = new_string(&obj_bytes(obj));
                // SAFETY: the substituted object is owned; the new string has
                // independent bytes and starts its own native count recipe.
                unsafe {
                    obj::incr_ref_count(copy);
                    obj::decr_ref_count(obj);
                }
                copy
            } else {
                obj
            };
            if w.expand {
                // Expand the live list objects directly. Stringifying and
                // reparsing would lose numeric/byte representations and reject
                // raw Jim strings already held in valid list elements.
                let elems = match crate::list::list_elements(obj) {
                    Ok(e) => e,
                    Err(e) => {
                        unsafe { obj::decr_ref_count(obj) };
                        release_all(&argv);
                        return self.report_cmd_error(e.into());
                    }
                };
                // For a `{*}` of a *literal* word, each element keeps its source
                // line (its offset within the literal — C's `TclListLines`), so a
                // body-defining element (`namespace {*}{eval ns {proc …}}`) is
                // `type source`. A dynamic `{*}$v` has no per-element location.
                let offsets = is_literal
                    .then(|| scan_list_offsets(&obj_bytes(obj)))
                    .flatten();
                let parsed = offsets
                    .as_ref()
                    .is_some_and(|o| o.iter().all(|&(_, lit)| lit));
                compiled &= parsed;
                for (k, &eo) in elems.iter().enumerate() {
                    unsafe { obj::incr_ref_count(eo) };
                    let idx = argv.len();
                    argv.push(eo);
                    let (nl, lit) = offsets
                        .as_ref()
                        .and_then(|o| o.get(k))
                        .copied()
                        .unwrap_or((0, false));
                    arg_lines.push(word_line + nl);
                    if file.is_some() && lit {
                        labc.push(idx);
                    }
                    literals.push(idx);
                }
                unsafe { obj::decr_ref_count(obj) };
            } else {
                let idx = argv.len();
                argv.push(obj); // already owned (+1)
                arg_lines.push(word_line);
                if file.is_some() && is_literal {
                    labc.push(idx);
                }
                if is_literal {
                    literals.push(idx);
                }
            }
        }

        if argv.is_empty() {
            // TclEvalObjvInternal's empty expanded argv is a successful empty
            // command. It still resets the prior interpreter result, just as
            // the ordinary dispatch boundary below does.
            self.set_result_bytes(b"");
            return Code::Ok;
        }

        // TIP 280 LABC: record each literal argument's obj → (file, line) so a
        // later `eval`/`uplevel`/`proc`-body of that obj reports `type source`.
        // Popped after dispatch (dynamic scope; the objs live until then).
        let pushed = labc.len();
        if pushed > 0 {
            let mut locs = self.arg_locs.borrow_mut();
            for &idx in &labc {
                locs.push((argv[idx], file.clone(), arg_lines[idx]));
                if self.uses_jim_error_stack() {
                    obj::retain_script_location(argv[idx], file.clone(), arg_lines[idx]);
                }
            }
        }
        *self.arg_lines.borrow_mut() = arg_lines;
        if !compiled || literals.first() != Some(&0) {
            literals.clear();
        }
        let compiled_pushed = literals.len();
        if compiled_pushed > 0 {
            let base = argv.as_ptr() as usize;
            self.compiled_literals
                .borrow_mut()
                .extend(literals.iter().map(|&idx| (base, idx)));
        }

        let code = self.dispatch_native_selection(selected, &argv);
        if compiled_pushed > 0 {
            let mut literals = self.compiled_literals.borrow_mut();
            let keep = literals.len() - compiled_pushed;
            literals.truncate(keep);
        }
        if pushed > 0 {
            let mut locs = self.arg_locs.borrow_mut();
            let keep = locs.len() - pushed;
            locs.truncate(keep);
        }
        // Safe to release argv now: a command that made an argv element its
        // result did so via set_obj_result, which holds an independent +1.
        release_all(&argv);
        code
    }
}
