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

//! CFG construction from structured IR.
//!
//! Flattens structured IR (`If`, `For`, `While`, `Switch`, `Catch`,
//! `Try`) into a graph of basic blocks connected by terminators.
//! The per-construct lowering methods live in [`cfg_lower`].
//!
//! Public API:
//! - [`build_cfg`] — build CFGs for a whole module (top-level + procs).
//! - [`build_cfg_function`] — build a CFG for a single script body.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

use rustc_hash::FxHashMap;
use tcl_lexer::{Span, TokenType};
use tcl_registry::Traits;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::ingress::static_context_for;

use crate::cfg::{Block, BlockId, CfgModule, Function, LoopNode, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Module, Script, Statement};
use crate::ir_helpers::defs_from_ir_script;
use crate::naming::normalise_var_name;

use self::global_write_info::GlobalWriteInfo;
use self::upvar_info::{FrameReach, UpvarInfo, collect_upvar_targets};

/// Choose the [`CommandTokens`] for a "frozen" `while`/`for` runtime call.
///
/// The frozen-loop barrier hands the source words (the condition expression
/// text, the body script text) to the runtime `while` / `for` builtin, which
/// re-evaluates the condition and body on each iteration. Each word must be
/// pushed *as written*: a braced `{cond}` / `{body}` word verbatim
/// (substitution suppressed), a `$body` word interpolated. The codegen's
/// [`emit_call`] decides this from `argv_kinds[i] == Str && single_token_word`,
/// so the barrier needs the real per-word kinds.
///
/// `source` is the loop's recorded [`Statement::While::raw_tokens`] — the exact
/// segmenter token metadata. When present (always, for lowered loops) it is
/// used directly; the all-verbatim fallback ([`all_str_tokens`]) only covers
/// synthetically-constructed loops that carry no token metadata, matching the
/// braced-word idiom they are built from. Without this the words are pushed as
/// interpolated literals and the condition's command substitution is evaluated
/// *once* at the call site, freezing the loop (`while {[gets $f line] >= 0}`
/// happens to work, but a bare `while {[string length $x]}` would spin forever).
fn frozen_loop_tokens(cmd: &str, args: &[String], source: Option<&CommandTokens>) -> CommandTokens {
    source.cloned().unwrap_or_else(|| all_str_tokens(cmd, args))
}

/// Return `tokens` with the word at `idx` removed from every per-word vector.
///
/// Used to derive a `dict for`/`map` barrier's tokens from the source
/// `dict for {vars} $d {body}` tokens by dropping the `for`/`map` subcommand
/// word (index 1), so the remaining words line up with the barrier's
/// `::tcl::dict::for {vars} $d {body}` argv.
fn drop_word(tokens: &CommandTokens, idx: usize) -> CommandTokens {
    let mut out = tokens.clone();
    if idx < out.argv.len() {
        out.argv.remove(idx);
    }
    if idx < out.argv_texts.len() {
        out.argv_texts.remove(idx);
    }
    if idx < out.word_exprs.len() {
        out.word_exprs.remove(idx);
    }
    if idx < out.argv_kinds.len() {
        out.argv_kinds.remove(idx);
    }
    if idx < out.single_token_word.len() {
        out.single_token_word.remove(idx);
    }
    if let Some(expand) = out.expand_word.as_mut()
        && idx < expand.len()
    {
        expand.remove(idx);
    }
    debug_assert!(out.words_align_with_argv_text());
    out
}

/// Synthesise [`CommandTokens`] marking every word as a single brace-string
/// token — the verbatim fallback for a frozen loop with no recorded source
/// tokens. See [`frozen_loop_tokens`].
fn all_str_tokens(cmd: &str, args: &[String]) -> CommandTokens {
    let word_count = 1 + args.len();
    let mut argv_texts = Vec::with_capacity(word_count);
    argv_texts.push(cmd.to_owned());
    argv_texts.extend(args.iter().cloned());
    CommandTokens::from_lossy_parts(
        vec![Span::new(0, 0); word_count],
        argv_texts,
        vec![TokenType::Str; word_count],
        vec![true; word_count],
        Vec::new(),
        None,
    )
}

mod cfg_lower;
pub mod global_write_info;
pub mod upvar_info;

/// Mutable block used during construction, frozen into [`Block`] at the end.
struct MutableBlock {
    name: String,
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

/// Builder that accumulates blocks and loop metadata for one function.
pub(crate) struct CfgBuilder {
    counter: u32,
    blocks: HashMap<String, MutableBlock>,
    /// Block name → [`BlockId`], assigned in block-creation order so the
    /// frozen [`Function`]'s interner reflects that order.
    block_ids: FxHashMap<String, BlockId>,
    loop_nodes: HashMap<String, LoopNode>,
    inline_loops: bool,
    /// Map from command name to upvar summary, used to pre-populate
    /// caller-side `defs` on calls to procs that use `upvar`.  Empty
    /// when the builder is constructed without an upvar context
    /// (e.g. for one-off CFGs that don't have a Module to scan).
    upvar_procs: HashMap<String, UpvarInfo>,
    /// Map from command name to parameter list, used by the upvar
    /// wiring to resolve param-based upvar sources (`upvar 1 $param
    /// local`) against the actual call-site argument.
    proc_params: HashMap<String, Vec<String>>,
    /// Map from command name to the outer-scope (global/namespace) variable
    /// names that proc's body writes (see [`global_write_info`]), used to
    /// widen caller-side `defs` at a call site exactly like `upvar_procs` —
    /// so SCCP treats a global/namespace name as overdefined across an
    /// opaque call that writes it, not as an ordinary untouched local.
    global_write_procs: HashMap<String, GlobalWriteInfo>,
    /// Stack of `(break_target, continue_target)` block names for the
    /// enclosing loops, so `break` / `continue` in a body lower to a CFG
    /// edge.  Without this a `while 1 { … break }` exit block is
    /// unreachable and O107 false-fires on the code after the loop.
    loop_stack: Vec<(String, String)>,
    /// `try` body→handler exception edges (analysis builds only).
    exception_edges: Vec<(String, String)>,
    /// When `true`, record [`Self::exception_edges`] in `lower_try`.  Off for
    /// codegen builds so the default bytecode is unchanged.
    faithful_exceptions: bool,
    /// When `Some`, every block that raises an explicit `error` / `throw`
    /// is recorded here. `lower_try` installs a fresh list around its body
    /// so the on-error edge is sourced from each throw point (at its
    /// throw-time SSA versions) rather than the pre-`try` block — a
    /// body-set var is defined at a later throw.
    throw_blocks: Option<Vec<String>>,
    /// The block where the most recent `lower_script` call's straight-line
    /// control finally terminated, when that script had no normal
    /// fall-through (e.g. a trailing `return` / `error`). `None` when the
    /// script fell through. `lower_try` reads this to source an on-error
    /// edge from a body that terminated without an explicit `error`/`throw`
    /// (a bare `return`).
    last_terminal_block: Option<String>,
    /// Registry-described error contexts for inlined command bodies flattened
    /// into the function currently being built. Drained into
    /// [`crate::cfg::Function`] for source-text-independent error-frame
    /// emission.
    inline_body_error_sites: Vec<crate::cfg::InlineBodyErrorSite>,
    /// Caller-frame injection the function currently being built is subject
    /// to: the union of every called procedure's
    /// [`UpvarInfo::caller_frame_barrier`].  Drained into
    /// [`crate::cfg::Function::caller_frame_barrier`], which
    /// [`crate::dynamic_names::dynamic_name_barrier`] folds into the
    /// function's blindness lattice.
    caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier,
    /// Caller-frame names a callee may touch through an `upvar` alias or
    /// `uplevel` write — see [`CfgBuilder::record_alias_observed`].  Drained
    /// into [`crate::cfg::Function::alias_observed_vars`].
    alias_observed_vars: std::collections::BTreeSet<String>,
    /// Current `lower_script` recursion depth, bounded by [`MAX_LOWER_DEPTH`]
    /// so deeply-nested bodies cannot overflow the stack.
    depth: u32,
    /// The document's lexer configuration.  The embedded-substitution scans
    /// below (`command_heads_in_text`, `upvar_defs_from_text`,
    /// `global_write_defs_from_text`, `builtin_write_defs_from_text`) re-lex
    /// an IR statement's own argument text to find its `[…]` tokens and split
    /// each one into words, so they must read it under the grammar the
    /// document was lexed with — under `f5-irules` `}{` separates two words,
    /// under Jim `$(…)` is one, under 8.4 `{*}` is literal — or the head they
    /// credit is not the head that runs. Defaults to
    /// `LexerConfig::default()`; the pipeline threads the document's config
    /// through [`build_cfg_with_config`] and friends.
    config: tcl_lexer::LexerConfig,
}

/// Maximum nesting depth for the recursive `lower_script` descent.
/// `lower_script` ↔ `lower_if`/`lower_for`/`lower_while`/`lower_foreach`/
/// `lower_switch`/`lower_try` are mutually recursive with one Rust frame
/// per nesting level; deeply-nested (generated / minified) bodies would
/// otherwise overflow the stack — an uncatchable SIGABRT that takes down
/// the LSP worker or the `tcl` CLI. At the cap we stop descending and
/// treat the over-deep script as a non-fall-through tail, yielding a
/// truncated-but-valid CFG instead of a crash. No real source nests
/// anywhere near this.
///
/// Shares [`crate::depth_guard::MAX_SOURCE_NEST_DEPTH`] with the lowering
/// and analyser body walks — one budget, derived from a measured per-level
/// stack cost, rather than three copies of a hand-picked 256 (issue #1654).
///
/// Through the normal pipeline this cap is **unreachable**: `lowering`'s
/// copy of the same number already bounds the IR handed here, so a
/// `Module` built from source cannot nest deeper than the cap in the first
/// place. It matters for a host that builds a `Module` some other way, and
/// its 8,288 bytes a level is what the budget must still afford for that
/// caller — the same defence-in-depth role the optimiser and codegen walks
/// play over the same IR.
const MAX_LOWER_DEPTH: tcl_core_types::RecursionLimit = crate::depth_guard::MAX_SOURCE_NEST_DEPTH;

/// Maximum nested-`[...]` descent depth for
/// [`CfgBuilder::upvar_defs_from_text_bounded`] /
/// [`CfgBuilder::global_write_defs_from_text_bounded`] (issue #923 idx
/// 122) — bounds pathological/adversarial bracket nesting the same way
/// [`MAX_LOWER_DEPTH`] bounds `lower_script`. No real source nests
/// anywhere near this.
const MAX_EMBEDDED_SUBST_DEPTH: u32 = 16;

impl CfgBuilder {
    fn new(inline_loops: bool) -> Self {
        Self::new_with_upvars(inline_loops, HashMap::new(), HashMap::new(), HashMap::new())
    }

    fn new_with_upvars(
        inline_loops: bool,
        upvar_procs: HashMap<String, UpvarInfo>,
        proc_params: HashMap<String, Vec<String>>,
        global_write_procs: HashMap<String, GlobalWriteInfo>,
    ) -> Self {
        Self {
            counter: 0,
            blocks: HashMap::new(),
            block_ids: FxHashMap::default(),
            loop_nodes: HashMap::new(),
            inline_loops,
            upvar_procs,
            proc_params,
            global_write_procs,
            loop_stack: Vec::new(),
            exception_edges: Vec::new(),
            faithful_exceptions: false,
            throw_blocks: None,
            last_terminal_block: None,
            inline_body_error_sites: Vec::new(),
            caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier::default(),
            alias_observed_vars: std::collections::BTreeSet::new(),
            depth: 0,
            // dialect-drift-ok: the builder's own default; the pipeline
            // replaces it via `with_lexer_config` from the unit's config.
            config: tcl_lexer::LexerConfig::default(),
        }
    }

    /// Enable `try` exception-edge recording (analysis builds).
    fn with_faithful_exceptions(mut self) -> Self {
        self.faithful_exceptions = true;
        self
    }

    /// Read every embedded `[…]` substitution under the document's own
    /// grammar (see [`Self::config`]).
    fn with_lexer_config(mut self, config: tcl_lexer::LexerConfig) -> Self {
        self.config = config;
        self
    }

    /// Augment a statement's effective `defs` with caller-side
    /// variable names that any callee proc will modify via `upvar`.
    /// Returns a list of statements — the original (possibly with
    /// merged `defs` for the direct-call form) plus an optional
    /// synthetic `<upvar-invalidate>` `Statement::Call` prepended
    /// when the embedded-substitution form contributes defs that
    /// can't be merged into the host statement (e.g. an
    /// `AssignValue` whose `value` text contains `[upvar_proc arg]`).
    ///
    /// Direct-call form: looks up `Statement::Call::command` in
    /// `upvar_procs`; if found, merges
    /// `UpvarInfo::caller_side_defs(args, params)` into the call's
    /// own `defs`.
    ///
    /// Embedded-substitution form: scans the call's args / the
    /// `AssignValue`'s value text for `[command_substitution]`
    /// tokens whose head is a known upvar proc; merges those defs
    /// into the host Call when possible, or emits a synthetic
    /// `<upvar-invalidate>` Call before a non-Call host.
    ///
    /// The same two forms also widen `defs` with `global_write_procs`
    /// (a callee that writes an outer-scope name via `global`/`variable`/
    /// `upvar #0`, see [`global_write_info`]) — a global name doesn't
    /// depend on call-site arguments, so no params-based mapping is
    /// needed, just the literal name list.
    fn apply_upvar_invalidation(&mut self, stmt: Statement) -> Vec<Statement> {
        self.record_caller_frame_barrier(&stmt);
        self.record_alias_observed(&stmt);
        self.upvar_invalidated(stmt)
    }

    /// Record every caller-frame name a callee invoked by `stmt` may touch
    /// through an `upvar` alias or an `uplevel` write, so the dead-store /
    /// unused-assignment passes never delete a store such a callee can
    /// observe (`set callervar 5; get` where `get` runs `upvar 1 callervar
    /// m; return $m` — issue #1193's upvar differential).  The names land on
    /// [`crate::cfg::Function::alias_observed_vars`]; recording them as
    /// *reads* on the call statement instead would fabricate
    /// read-before-set uses (a false W210) for the pure out-param shape.
    fn record_alias_observed(&mut self, stmt: &Statement) {
        if self.upvar_procs.is_empty() {
            return;
        }
        if let Statement::Call { command, args, .. } = stmt
            && let Some(info) = self.upvar_procs.get(command.as_str())
        {
            let params: &[String] = self
                .proc_params
                .get(command.as_str())
                .map_or(&[][..], Vec::as_slice);
            self.alias_observed_vars
                .extend(info.caller_side_defs(args, params));
        }
        let texts: Vec<&str> = match stmt {
            Statement::AssignValue { value, .. }
            | Statement::Return {
                value: Some(value), ..
            } if value.contains('[') => vec![value.as_str()],
            Statement::Call { args, .. } => args
                .iter()
                .filter(|a| a.contains('['))
                .map(String::as_str)
                .collect(),
            _ => Vec::new(),
        };
        for text in texts {
            self.alias_observed_vars
                .extend(self.upvar_defs_from_text(text));
        }
    }

    /// Whole-frame blindness the statement's calls impose on *this*
    /// function, as distinct from the per-name `defs` widening
    /// [`Self::upvar_invalidated`] applies.
    ///
    /// A callee that runs `uplevel 1 $body` — or aliases a caller-frame name
    /// it cannot place — reaches names no `defs` list can enumerate.
    /// Recorded flow-insensitively on the function, in the same lattice and
    /// the same abstention direction as the dynamic-name barrier
    /// ([`crate::dynamic_names`]).
    fn record_caller_frame_barrier(&mut self, stmt: &Statement) {
        for command in Self::called_command_names(stmt, self.config) {
            if let Some(info) = self.upvar_procs.get(command.as_str()) {
                self.caller_frame_barrier =
                    self.caller_frame_barrier.union(info.caller_frame_barrier());
            }
        }
    }

    /// The `defs`-widening half of [`Self::apply_upvar_invalidation`], with
    /// no side effect on the function-level barrier — so a speculative query
    /// ([`Self::init_written_names`]) can ask what a statement writes without
    /// recording the statement twice.
    fn upvar_invalidated(&self, mut stmt: Statement) -> Vec<Statement> {
        // 0. A callee whose caller-frame effect cannot be enumerated per
        //    name widens the call site with an opaque barrier after the
        //    call (SCCP/propagation widen every tracked value at a
        //    `Statement::Barrier`), instead of trusting the
        //    under-approximate `caller_side_defs` / `names`.
        if let Some(barrier) = self.opaque_call_barrier(&stmt) {
            return vec![stmt, barrier];
        }

        // 1. Direct-call extras: command is a known upvar proc / a proc
        //    that writes outer-scope names.
        let direct_extras = self.direct_call_extras(&stmt);

        // 2. Embedded-substitution extras: walk text for
        //    `[upvar_proc arg]` / `[global_write_proc arg]` substitutions.
        let (embedded_extras, embedded_opaque_global) = self.embedded_subst_extras(&stmt);

        if direct_extras.is_empty() && embedded_extras.is_empty() && !embedded_opaque_global {
            return vec![stmt];
        }

        // 2b. An embedded call to a proc that runs an unreadable script at
        //     the global frame (`set y [setter]` where `setter` does
        //     `uplevel #0 $body`, issue #1198): no def list can enumerate
        //     what it clobbers, so prepend an opaque barrier — the same
        //     program-order position the synthetic `<upvar-invalidate>`
        //     uses, so the host statement's own reads already see the
        //     widened state.
        let opaque_barrier = embedded_opaque_global.then(|| Statement::Barrier {
            span: stmt.span(),
            reason: "embedded call runs an unreadable script at the global frame".to_owned(),
            command: "<global-frame-script>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::GlobalFrameScript,
            )),
        });

        // 3. Merge into the host statement when it's a Call.
        if let Statement::Call { defs, .. } = &mut stmt {
            for d in direct_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            for d in embedded_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            return match opaque_barrier {
                Some(barrier) => vec![barrier, stmt],
                None => vec![stmt],
            };
        }

        // 4. Non-Call host (e.g. AssignValue) with embedded extras —
        //    emit a synthetic `<upvar-invalidate>` Call before the
        //    host so the affected vars are invalidated in
        //    program order.
        let mut out = Vec::new();
        if let Some(barrier) = opaque_barrier {
            out.push(barrier);
        }
        if !embedded_extras.is_empty() {
            out.push(Statement::Call {
                span: stmt.span(),
                command: "<upvar-invalidate>".to_string(),
                canonical_command: None,
                args: Vec::new(),
                defs: embedded_extras,
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::UpvarInvalidate,
                )),
                foreach_groups: None,
            });
        }
        out.push(stmt);
        out
    }

    /// The opaque widening barrier for a direct call whose callee's
    /// caller-frame effect has no sound per-name def list: a callee whose
    /// `upvar` caller-side name is unresolvable (`upvar 1 $computed x`) can
    /// write ANY caller variable, and a callee that runs an unreadable
    /// script at the global frame (`uplevel #0 $body`, issue #1198) can
    /// write or read ANY global/namespace name.
    fn opaque_call_barrier(&self, stmt: &Statement) -> Option<Statement> {
        let Statement::Call { command, span, .. } = stmt else {
            return None;
        };
        let unresolvable_upvar = self
            .upvar_procs
            .get(command.as_str())
            .is_some_and(|info| info.has_unresolvable_caller_target);
        let opaque_global = self
            .global_write_procs
            .get(command.as_str())
            .is_some_and(|info| info.opaque_global_frame);
        if !unresolvable_upvar && !opaque_global {
            return None;
        }
        let reason = if unresolvable_upvar {
            format!("{command} upvar-aliases a dynamic caller variable")
        } else {
            format!("{command} runs an unreadable script at the global frame")
        };
        Some(Statement::Barrier {
            span: *span,
            // A widening *effect*, not a command to run: the call itself is
            // already in the statement stream immediately beside this barrier,
            // so naming the callee here made codegen invoke it a second time
            // (issue #1602 — `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p`
            // printed `u=…` twice on the VM where tclsh 8.6.14 / 9.0.4 print it
            // once; `proc setter {body} { uplevel #0 $body }; setter {set q 1}`
            // failed with `wrong # args` from the re-invoke). The typed
            // `SyntheticMarker` on the tokens is what stops codegen
            // dispatching it; the `command` spelling below is a label for the
            // disassembly and the explorer, and `reason` keeps the callee's
            // name.
            reason,
            command: "<caller-frame-opaque>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::CallerFrameOpaque,
            )),
        })
    }

    /// The direct-call half of [`Self::upvar_invalidated`]: the caller-side
    /// names a known upvar proc / global-writing proc defines at this call.
    /// Extras are merged as DEFS only; the "the callee may also *read* the
    /// aliased cell" half is recorded on
    /// [`crate::cfg::Function::alias_observed_vars`] by
    /// [`Self::record_alias_observed`] (a read here would fabricate
    /// read-before-set uses — a false W210 — for the pure out-param shape).
    fn direct_call_extras(&self, stmt: &Statement) -> Vec<String> {
        let Statement::Call { command, args, .. } = stmt else {
            return Vec::new();
        };
        let mut extras = self
            .upvar_procs
            .get(command.as_str())
            .map(|info| {
                let params: &[String] = self
                    .proc_params
                    .get(command.as_str())
                    .map_or(&[][..], Vec::as_slice);
                info.caller_side_defs(args, params)
            })
            .unwrap_or_default();
        if let Some(info) = self.global_write_procs.get(command.as_str()) {
            for name in &info.names {
                if !extras.contains(name) {
                    extras.push(name.clone());
                }
            }
        }
        extras
    }

    /// The embedded-substitution half of [`Self::upvar_invalidated`]: the
    /// caller-side defs contributed by `[…]` substitutions in the
    /// statement's argument words (or an assignment's value), plus whether
    /// any embedded callee runs an unreadable script at the global frame.
    fn embedded_subst_extras(&self, stmt: &Statement) -> (Vec<String>, bool) {
        let texts: Vec<&str> = match stmt {
            Statement::AssignValue { value, .. } if value.contains('[') => vec![value.as_str()],
            Statement::Call { args, .. } => args
                .iter()
                .filter(|a| a.contains('['))
                .map(String::as_str)
                .collect(),
            _ => Vec::new(),
        };
        let mut embedded_extras: Vec<String> = Vec::new();
        let mut embedded_opaque_global = false;
        for text in texts {
            for d in self.upvar_defs_from_text(text) {
                if !embedded_extras.contains(&d) {
                    embedded_extras.push(d);
                }
            }
            let (global_defs, opaque) = self.global_write_defs_from_text(text);
            embedded_opaque_global |= opaque;
            for d in global_defs {
                if !embedded_extras.contains(&d) {
                    embedded_extras.push(d);
                }
            }
            // A var-mutating builtin inside a command substitution
            // (`set y [append x b]`, `[incr x]`, `[lset l …]`) writes its
            // target variable as a side effect; record it so copy / constant
            // propagation (O100) does not propagate a stale value past the
            // mutation (FP-OPT-06).
            for d in Self::builtin_write_defs_from_text(text, self.config) {
                if !embedded_extras.contains(&d) {
                    embedded_extras.push(d);
                }
            }
        }
        (embedded_extras, embedded_opaque_global)
    }

    /// Every command word a statement invokes: its own head, plus the head
    /// of every `[…]` substitution nested in its arguments (or, for an
    /// assignment, in its value).
    ///
    /// The caller-frame barrier is a *whole-function* fact, so it has to be
    /// raised wherever a call appears — `helper $x`, `set y [helper $x]`,
    /// and `if {[helper $x]} …` alike. Bounded by
    /// [`MAX_EMBEDDED_SUBST_DEPTH`], like every other bracket descent here.
    fn called_command_names(stmt: &Statement, config: tcl_lexer::LexerConfig) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut texts: Vec<&str> = Vec::new();
        match stmt {
            Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
                out.push(command.clone());
                texts.extend(args.iter().map(String::as_str));
            }
            Statement::AssignValue { value, .. } | Statement::AssignConst { value, .. } => {
                texts.push(value.as_str());
            }
            _ => {}
        }
        for text in texts {
            Self::command_heads_in_text(text, 0, &mut out, config);
        }
        out
    }

    /// Accumulate the head word of every `[…]` substitution in *text*,
    /// descending into nested brackets.
    fn command_heads_in_text(
        text: &str,
        depth: u32,
        out: &mut Vec<String>,
        config: tcl_lexer::LexerConfig,
    ) {
        use tcl_lexer::{Lexer, SourceMap, TokenType};

        if !text.contains('[') || depth >= MAX_EMBEDDED_SUBST_DEPTH {
            return;
        }
        let sm = SourceMap::new(text);
        let Ok(tokens) = Lexer::with_config(text, config).tokenise_all() else {
            return;
        };
        for tok in &tokens {
            if tok.kind != TokenType::Cmd {
                continue;
            }
            let inner = sm.token_text(*tok);
            if let Some(head) = words_from_text_with_config(inner, config).first() {
                out.push(head.clone());
            }
            Self::command_heads_in_text(inner, depth + 1, out, config);
        }
    }

    /// Scan *text* for `[command_substitution]` tokens and
    /// accumulate caller-side defs from any embedded calls to
    /// known upvar procs.
    fn upvar_defs_from_text(&self, text: &str) -> Vec<String> {
        self.upvar_defs_from_text_bounded(text, 0)
    }

    /// [`Self::upvar_defs_from_text`], recursing into a nested `[...]`
    /// found within a matched command's own inner text (issue #923 idx
    /// 122): `set err [getopt argv $opts opt arg]` — the real tcllib
    /// `cmdline::getoptions` shape, condition-position or not — wraps the
    /// known upvar proc call in an outer `set`, so only checking
    /// `words.first()` against `upvar_procs` misses it entirely (`set`
    /// itself is never a known upvar proc). `words_from_text` strips a
    /// nested `Cmd` token's own brackets when flattening it into a word
    /// (mirroring `SourceMap::token_text`'s "extract inner content"
    /// convention), so the nested substitution can only be recovered by
    /// re-lexing `inner` itself, not by pattern-matching the word list.
    /// `depth` bounds the descent the same way [`MAX_LOWER_DEPTH`] bounds
    /// `lower_script`, against pathological/adversarial nesting.
    fn upvar_defs_from_text_bounded(&self, text: &str, depth: u32) -> Vec<String> {
        use tcl_lexer::{Lexer, SourceMap, TokenType};

        if self.upvar_procs.is_empty() || !text.contains('[') || depth >= MAX_EMBEDDED_SUBST_DEPTH {
            return Vec::new();
        }

        let sm = SourceMap::new(text);
        let lexer = Lexer::with_config(text, self.config);
        let Ok(tokens) = lexer.tokenise_all() else {
            return Vec::new();
        };

        let mut defs: Vec<String> = Vec::new();
        for tok in &tokens {
            if tok.kind != TokenType::Cmd {
                continue;
            }
            // Inner text of `[...]`, re-lexed for word extraction.
            let inner = sm.token_text(*tok);
            let words = words_from_text_with_config(inner, self.config);
            if let Some(cmd) = words.first()
                && let Some(info) = self.upvar_procs.get(cmd.as_str())
            {
                let params: &[String] = self
                    .proc_params
                    .get(cmd.as_str())
                    .map_or(&[][..], Vec::as_slice);
                let raw_args: Vec<String> = words.iter().skip(1).cloned().collect();
                for d in info.caller_side_defs(&raw_args, params) {
                    if !defs.contains(&d) {
                        defs.push(d);
                    }
                }
            }
            // `inner`'s own raw text still carries any bracket nested
            // inside it (only the outermost pair this token wraps was
            // stripped) — recurse on it, one level shallower, so a
            // wrapping command doesn't hide a known upvar proc buried
            // inside it.
            for d in self.upvar_defs_from_text_bounded(inner, depth + 1) {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
        }
        defs
    }

    /// Scan *text* for `[command_substitution]` tokens and accumulate the
    /// outer-scope (global/namespace) names any embedded call to a known
    /// global-writing proc writes — the embedded-substitution analogue of
    /// [`Self::upvar_defs_from_text`], for the same soundness reason: a
    /// global write reached via `set y [mutate]` is just as real as one
    /// reached via a bare `mutate` statement.
    ///
    /// The second return is `true` when any embedded callee's summary
    /// carries [`GlobalWriteInfo::opaque_global_frame`] (`uplevel #0 $body`,
    /// issue #1198) — no def list can enumerate that, so the caller must
    /// widen with an opaque barrier.
    fn global_write_defs_from_text(&self, text: &str) -> (Vec<String>, bool) {
        let mut opaque = false;
        let defs = self.global_write_defs_from_text_bounded(text, 0, &mut opaque);
        (defs, opaque)
    }

    /// [`Self::global_write_defs_from_text`], recursing into a nested
    /// `[...]` the same way [`Self::upvar_defs_from_text_bounded`] does
    /// (issue #923 idx 122) — see its doc for the wrapping-command shape
    /// this recovers.
    fn global_write_defs_from_text_bounded(
        &self,
        text: &str,
        depth: u32,
        opaque: &mut bool,
    ) -> Vec<String> {
        use tcl_lexer::{Lexer, SourceMap, TokenType};

        if self.global_write_procs.is_empty()
            || !text.contains('[')
            || depth >= MAX_EMBEDDED_SUBST_DEPTH
        {
            return Vec::new();
        }

        let sm = SourceMap::new(text);
        let lexer = Lexer::with_config(text, self.config);
        let Ok(tokens) = lexer.tokenise_all() else {
            return Vec::new();
        };

        let mut defs: Vec<String> = Vec::new();
        for tok in &tokens {
            if tok.kind != TokenType::Cmd {
                continue;
            }
            let inner = sm.token_text(*tok);
            let words = words_from_text_with_config(inner, self.config);
            if let Some(cmd) = words.first()
                && let Some(info) = self.global_write_procs.get(cmd.as_str())
            {
                *opaque |= info.opaque_global_frame;
                for name in &info.names {
                    if !defs.contains(name) {
                        defs.push(name.clone());
                    }
                }
            }
            // See `upvar_defs_from_text_bounded`'s identical step: `inner`
            // still carries any bracket nested inside it, so recurse on
            // the raw text rather than the (bracket-stripped) word list.
            for d in self.global_write_defs_from_text_bounded(inner, depth + 1, opaque) {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
        }
        defs
    }

    /// Condition-position command-substitution out-vars (issue #923 idx
    /// 122): unions the registry's `ArgRole::VarWrite` scan
    /// ([`crate::ir_helpers::condition_command_out_vars`]) with the same
    /// known-upvar-proc /
    /// known-global-writer resolution every *other* embedded-substitution
    /// site already gets ([`Self::upvar_defs_from_text`] /
    /// [`Self::global_write_defs_from_text`]).  Without this, a user
    /// proc's `upvar` write was only recognised as a bare statement or an
    /// ordinary value (`set x [getKnownOpt ...]`) — invoked from a
    /// `while`/`if` *condition* instead (`while {[getopt argv $opts opt
    /// arg]} { ... }`, tcllib's `cmdline::getoptions`), the write was
    /// invisible, producing a false W210 on the guarded body's read even
    /// though the condition's own command substitution (including the
    /// upvar write) completes before the body ever runs
    /// (tclsh9.0/8.6-verified).
    /// The second return is `true` when a condition-embedded callee runs an
    /// unreadable script at the global frame (issue #1198): the caller must
    /// then push an opaque barrier alongside the `<cond>` defs, because no
    /// def list can enumerate what the condition's evaluation clobbers.
    fn condition_out_vars(&self, condition: &ExprNode) -> (Vec<String>, bool) {
        // The CFG builder carries no registry handle (see the module note at
        // `registry_derived_cfg_classes`); the write-role answer is core Tcl in
        // every dialect, so the cached default registry is the right source.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut out = crate::ir_helpers::condition_command_out_vars(condition, registry);
        let mut opaque = false;
        let mut cmds = Vec::new();
        crate::ir_helpers::collect_expr_commands(condition, &mut cmds);
        for cmd_text in &cmds {
            for d in self.upvar_defs_from_text(cmd_text) {
                if !out.contains(&d) {
                    out.push(d);
                }
            }
            let (global_defs, cmd_opaque) = self.global_write_defs_from_text(cmd_text);
            opaque |= cmd_opaque;
            for d in global_defs {
                if !out.contains(&d) {
                    out.push(d);
                }
            }
        }
        (out, opaque)
    }

    /// Push the `<cond>` synthetic call (and, when the condition's embedded
    /// callees demand it, an opaque barrier) for a condition that contains
    /// command substitutions — the shared tail of `lower_if`, `lower_while`,
    /// and the frozen-loop barrier.
    fn push_condition_effects(&mut self, condition: &ExprNode, span: Span, block: &str) {
        let (cond_defs, opaque) = self.condition_out_vars(condition);
        if !cond_defs.is_empty() {
            self.block_mut(block).statements.push(Statement::Call {
                span,
                command: "<cond>".into(),
                canonical_command: None,
                args: Vec::new(),
                defs: cond_defs,
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::Condition,
                )),
                foreach_groups: None,
            });
        }
        if opaque {
            self.block_mut(block).statements.push(Statement::Barrier {
                span,
                reason: "condition runs an unreadable script at the global frame".into(),
                command: "<global-frame-script>".into(),
                canonical_command: None,
                args: Vec::new(),
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::GlobalFrameScript,
                )),
            });
        }
    }

    /// Scan *text* for `[command_substitution]` tokens whose head is a builtin
    /// command that mutates a named variable passed as a literal argument
    /// (`append` / `lappend` / `incr` / `lset` / `set`, and the `dict`
    /// sub-mutators). Returns the written variable names, so a copy / constant
    /// propagation pass treats the substitution as a kill-site for them. Only
    /// literal targets are returned — a `$`-substituted or computed target name
    /// is not statically known. Recurses into nested substitutions.
    fn builtin_write_defs_from_text(text: &str, config: tcl_lexer::LexerConfig) -> Vec<String> {
        use tcl_lexer::{Lexer, SourceMap, TokenType};

        if !text.contains('[') {
            return Vec::new();
        }
        let sm = SourceMap::new(text);
        let Ok(tokens) = Lexer::with_config(text, config).tokenise_all() else {
            return Vec::new();
        };

        let mut defs: Vec<String> = Vec::new();
        let record = |name: Option<&String>, defs: &mut Vec<String>| {
            if let Some(n) = name
                && !n.is_empty()
                && !n.contains('$')
                && !n.contains('[')
                && !defs.iter().any(|d| d == n)
            {
                defs.push(n.clone());
            }
        };
        for tok in &tokens {
            if tok.kind != TokenType::Cmd {
                continue;
            }
            let inner = sm.token_text(*tok);
            let words = words_from_text_with_config(inner, config);
            let Some(cmd) = words.first().map(String::as_str) else {
                continue;
            };
            // Membership is registry-driven: the first-arg writers via
            // `writes_first_arg_variable`, and the `dict` sub-mutators via
            // the per-subcommand `VarWrite` role (which includes the
            // body-carrying `update`/`with` — their bodies write the dict
            // back on completion, so the substitution is a kill-site).
            let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
            if registry.writes_first_arg_variable(cmd) {
                record(words.get(1), &mut defs);
            } else if words.len() >= 2 {
                let arg_strs: Vec<&str> = words[1..].iter().map(String::as_str).collect();
                if registry
                    .arg_indices_for_role(cmd, &arg_strs, tcl_registry::ArgRole::VarWrite)
                    .contains(&1)
                {
                    record(words.get(2), &mut defs);
                }
            }
            // Nested substitutions inside this one (`[set y [incr x]]`).
            for d in Self::builtin_write_defs_from_text(inner, config) {
                record(Some(&d), &mut defs);
            }
        }
        defs
    }

    /// If `stmt` is a loop jump (the registry's `BREAKS_LOOP` /
    /// `CONTINUES_LOOP` classes) inside a loop, push it into
    /// `current` and set a `Goto` terminator to the loop's exit / continue
    /// target, returning `true`.  Returns `false` (no-op) otherwise.
    /// Matched against the raw command word (no `::` trimming), as the
    /// retired hardcoded comparison was.
    fn lower_loop_jump(&mut self, current: &str, stmt: &Statement) -> bool {
        let Statement::Call { command, span, .. } = stmt else {
            return false;
        };
        let is_break = is_loop_break_command(command);
        if !is_break && !is_loop_continue_command(command) {
            return false;
        }
        let Some((brk, cont)) = self.loop_stack.last().cloned() else {
            return false;
        };
        let target_name = if is_break { brk } else { cont };
        let target = self.bid(&target_name);
        self.block_mut(current).statements.push(stmt.clone());
        self.block_mut(current).terminator = Some(Terminator::Goto {
            target,
            span: Some(*span),
        });
        true
    }

    /// Push a non-control-flow statement into `current` (after upvar
    /// invalidation), promoting `error` / `throw` / `exit` (and, in analysis
    /// builds, `tailcall`) to a `Return` terminator so any following statements
    /// become dead code (mirrors the `TERMINATES_BLOCK` registry trait).
    fn push_plain_statement(&mut self, current: &str, stmt: &Statement) {
        for s in self.apply_upvar_invalidation(stmt.clone()) {
            self.block_mut(current).statements.push(s);
        }
        if let Statement::Call {
            command,
            canonical_command,
            span,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            span,
            ..
        } = stmt
            && self.block_mut(current).terminator.is_none()
        {
            let canon = canonical_command.as_deref().unwrap_or(command);
            // `tailcall` (Tcl 8.6+, FP-RBS-13) replaces the current frame and
            // never returns here, so it ends straight-line flow exactly like
            // `error`/`exit`.  Promote it only in analysis builds
            // (`faithful_exceptions`) so the codegen / non-faithful CFG shape
            // stays byte-identical — codegen leaves the call as a fall-through.
            let exits_proc = is_block_terminating_command(canon)
                || (self.faithful_exceptions && is_tailcall_command(canon));
            if exits_proc {
                // A catchable `error` / `throw` (not `exit` / `tailcall`, which
                // leave the process / pop the frame) is a throw point: record
                // the current block so an enclosing `try`'s on-error edge can be
                // sourced from here, where the body's prior defs are live.
                if is_catchable_throw(canon)
                    && let Some(blocks) = self.throw_blocks.as_mut()
                {
                    blocks.push(current.to_owned());
                }
                self.block_mut(current).terminator = Some(Terminator::Return {
                    value: None,
                    span: Some(*span),
                    expr: None,
                    braced: false,
                });
            }
        }
    }

    /// Allocate a new empty block with a unique name, interning its
    /// [`BlockId`] in creation order.
    fn new_block(&mut self, prefix: &str) -> String {
        self.counter += 1;
        let name = format!("{prefix}_{}", self.counter);
        let id = BlockId(u32::try_from(self.block_ids.len()).expect("CFG block count fits in u32"));
        self.block_ids.insert(name.clone(), id);
        self.blocks.insert(
            name.clone(),
            MutableBlock {
                name: name.clone(),
                statements: Vec::new(),
                terminator: None,
            },
        );
        name
    }

    /// The [`BlockId`] interned for a created block name. Panics if the
    /// name was never created by [`Self::new_block`].
    fn bid(&self, name: &str) -> BlockId {
        *self
            .block_ids
            .get(name)
            .unwrap_or_else(|| panic!("block {name} has no id"))
    }

    /// Borrow a mutable block by name. Panics if missing.
    fn block_mut(&mut self, name: &str) -> &mut MutableBlock {
        self.blocks
            .get_mut(name)
            .unwrap_or_else(|| panic!("block {name} not found"))
    }

    /// Set a `Goto` terminator on a block only if it doesn't already
    /// have one.
    fn ensure_goto(&mut self, block_name: &str, target: &str, span: Option<Span>) {
        let target = self.bid(target);
        let block = self.block_mut(block_name);
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Goto { target, span });
        }
    }

    /// Build a [`Function`] by lowering a script starting at a fresh
    /// entry block, then freezing all mutable blocks.
    fn build_function(&mut self, name: &str, script: &Script) -> Function {
        let entry = self.new_block("entry");
        let tail = self.lower_script(script, &entry);
        if let Some(tail) = tail {
            let exit = self.new_block("exit");
            self.ensure_goto(&tail, &exit, None);
        }

        // Seed the function's interner in block-creation order (by `BlockId`),
        // so the frozen interner's ids match the ones the builder stamped into
        // every terminator / loop node / exception edge.
        let mut func = Function::new(name, &entry);
        let mut ordered: Vec<(BlockId, &String)> =
            self.block_ids.iter().map(|(n, id)| (*id, n)).collect();
        ordered.sort_by_key(|(id, _)| *id);
        for (_, n) in &ordered {
            func.intern_block((*n).clone());
        }

        func.blocks = self
            .blocks
            .drain()
            .map(|(k, mb)| {
                let id = func
                    .block_id(&k)
                    .unwrap_or_else(|| panic!("frozen block {k} not interned"));
                (
                    id,
                    Block {
                        name: mb.name,
                        statements: mb.statements,
                        terminator: mb.terminator,
                    },
                )
            })
            .collect();

        func.loop_nodes = std::mem::take(&mut self.loop_nodes)
            .into_iter()
            .map(|(k, ln)| (self.bid(&k), ln))
            .collect();
        func.exception_edges = std::mem::take(&mut self.exception_edges)
            .into_iter()
            .map(|(from, to)| (self.bid(&from), self.bid(&to)))
            .collect();
        func.inline_body_error_sites = std::mem::take(&mut self.inline_body_error_sites);
        func.caller_frame_barrier = self.caller_frame_barrier;
        func.alias_observed_vars = std::mem::take(&mut self.alias_observed_vars);
        func
    }

    /// Lower a script (sequence of IR statements) into CFG blocks.
    ///
    /// `block_name` is the block where the first statement lands.
    /// Returns `Some(tail_block)` — the block where subsequent code
    /// should go — or `None` if control doesn't fall through (e.g.
    /// the script ends with a `return`).
    /// Depth-guarded entry to the recursive lowering. Every
    /// nested body re-enters here, so bounding this one point caps the
    /// whole `lower_*` recursion. At the cap we stop descending and report
    /// "no fall-through" — `build_function` already handles a `None` tail,
    /// so the result is a truncated-but-valid CFG rather than a stack
    /// overflow.
    fn lower_script(&mut self, script: &Script, block_name: &str) -> Option<String> {
        self.depth += 1;
        if MAX_LOWER_DEPTH.exceeded(self.depth) {
            self.depth -= 1;
            return None;
        }
        let result = self.lower_script_inner(script, block_name);
        self.depth -= 1;
        result
    }

    /// Push a "frozen" loop (`for` / `while` whose condition is a command
    /// substitution) into `current` as an opaque [`Statement::Barrier`]; the
    /// body is kept un-lowered so it is treated as an opaque effect.
    ///
    /// A `Statement::Barrier` has no `defs` field — its own
    /// [`crate::ssa::uses_of`] textually scans *both* `condition` and the
    /// un-lowered body text for `$var` reads (attributing them all to the
    /// barrier's own span), but nothing analogous computes defs for it
    /// beyond the registry's ordinary `VarWrite` role query, which `for` /
    /// `while` themselves never declare. So a `catch`/`regexp`/`scan`
    /// result var, or a known upvar/global-writing user proc's write,
    /// reached only through the frozen condition (`while {[getopt argv
    /// $opts opt arg]} { ... }`, tcllib's `cmdline::getoptions` — issue
    /// #923 idx 122) was invisible to the def-use graph: the read inside
    /// the (un-lowered, but still textually-scanned) body looked
    /// read-before-set even though the condition's own command
    /// substitution completes — including the write — before the body
    /// (or even a second evaluation of the condition) ever runs. Fixed by
    /// pushing a synthetic `<cond>` `Statement::Call` carrying those defs
    /// immediately before the barrier, exactly mirroring the non-frozen
    /// `lower_if`/`lower_while` path's own `condition_out_vars` use — the
    /// barrier's textually-scanned read then resolves to this new SSA
    /// version instead of the undef origin.
    fn push_frozen_loop_barrier(
        &mut self,
        command: &str,
        condition: &ExprNode,
        raw_args: &[String],
        raw_tokens: Option<&CommandTokens>,
        span: Span,
        current: &str,
    ) {
        self.push_condition_effects(condition, span, current);
        self.block_mut(current).statements.push(Statement::Barrier {
            span,
            reason: format!("frozen {command} (cmd-subst condition)"),
            command: command.to_owned(),
            canonical_command: None,
            args: raw_args.to_vec(),
            tokens: Some(frozen_loop_tokens(command, raw_args, raw_tokens)),
        });
    }

    /// Lower a `for`, or freeze it as an opaque barrier when its condition is a
    /// command substitution (`for {…} [cond] {…} {…}`).
    fn lower_for_or_frozen(&mut self, stmt: &Statement, current: &str) -> Option<String> {
        if let Statement::For {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "for",
                condition,
                raw_args,
                raw_tokens.as_ref(),
                *span,
                current,
            );
            Some(current.to_owned())
        } else {
            self.lower_for(stmt, current)
        }
    }

    /// Lower a `while`, or freeze it as an opaque barrier when its condition is
    /// a command substitution (`while [cond] {…}`).
    fn lower_while_or_frozen(&mut self, stmt: &Statement, current: &str) -> String {
        if let Statement::While {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "while",
                condition,
                raw_args,
                raw_tokens.as_ref(),
                *span,
                current,
            );
            current.to_owned()
        } else {
            self.lower_while(stmt, current)
        }
    }

    /// `return -options …` / `return {*}…args`: push the original barrier
    /// (codegen keeps its raw args) but, in `faithful_exceptions` analysis
    /// builds, also terminate `current` with a `Return` so the fall-through
    /// edge to the rest of the block / `try` join is cut.
    fn lower_return_options_barrier(&mut self, stmt: &Statement, span: Span, current: &str) {
        self.push_plain_statement(current, stmt);
        self.block_mut(current).terminator = Some(Terminator::Return {
            value: None,
            span: Some(span),
            expr: None,
            braced: false,
        });
    }

    fn lower_return_statement(&mut self, stmt: &Statement, current: &str) {
        let Statement::Return {
            span,
            value,
            expr,
            braced,
        } = stmt
        else {
            unreachable!();
        };

        // `return [upvar_proc …]` invokes the callee before the frame unwinds,
        // so its caller-frame aliases must be widened exactly as for a plain
        // call. Otherwise the feeding store in this frame looks dead and
        // O109/O126 can delete it (issue #1193's upvar differential).
        self.record_alias_observed(stmt);
        if let Some(v) = value.as_deref()
            && v.contains('[')
        {
            let extras = self.upvar_defs_from_text(v);
            if !extras.is_empty() {
                let synthetic = Statement::Call {
                    span: *span,
                    command: "<upvar-invalidate>".to_string(),
                    canonical_command: None,
                    args: Vec::new(),
                    defs: extras,
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: Some(crate::ir::CommandTokens::marker(
                        crate::ir::SyntheticMarker::UpvarInvalidate,
                    )),
                    foreach_groups: None,
                };
                self.block_mut(current).statements.push(synthetic);
            }
        }
        self.block_mut(current).terminator = Some(Terminator::Return {
            value: value.clone(),
            span: Some(*span),
            expr: expr.clone(),
            braced: *braced,
        });
    }

    fn lower_script_statement(&mut self, stmt: &Statement, current: &str) -> Option<String> {
        match stmt {
            Statement::If { .. } => Some(self.lower_if(stmt, current)),
            Statement::For { .. } => self.lower_for_or_frozen(stmt, current),
            Statement::While { .. } => Some(self.lower_while_or_frozen(stmt, current)),
            Statement::Foreach { .. } => Some(self.lower_foreach_dispatch(stmt, current)),
            Statement::Catch { .. } => {
                self.emit_opaque_catch(stmt, current);
                Some(current.to_owned())
            }
            Statement::Try { .. } => Some(self.lower_try_dispatch(stmt, current)),
            Statement::Switch { .. } => Some(self.lower_switch(stmt, current)),
            Statement::Return { .. } => {
                self.lower_return_statement(stmt, current);
                Some(current.to_owned())
            }
            // Inline block: flatten the body's statements into the current
            // control-flow stream so SSA / codegen see them as plain inline
            // statements. Preserve any registry-described Tcl error context
            // with the original command span.
            Statement::Block {
                body,
                span,
                error_context,
                ..
            } => {
                if let Some(context) = error_context {
                    self.inline_body_error_sites
                        .push(crate::cfg::InlineBodyErrorSite {
                            span: *span,
                            context: *context,
                        });
                }
                self.lower_script(body, current)
            }
            // `return -options …` / `return {*}…args` lower to a
            // Statement::Barrier, but still unconditionally exit the proc
            // in analysis builds.
            Statement::Barrier { reason, span, .. }
                if self.faithful_exceptions
                    && matches!(
                        reason.as_str(),
                        "return with options" | "return with expansion"
                    ) =>
            {
                self.lower_return_options_barrier(stmt, *span, current);
                Some(current.to_owned())
            }
            other => {
                self.push_plain_statement(current, other);
                Some(current.to_owned())
            }
        }
    }

    fn lower_script_inner(&mut self, script: &Script, block_name: &str) -> Option<String> {
        let mut current = block_name.to_owned();
        // True once the *main* (reachable) path has hit an unconditional
        // terminator — everything after is dead code captured in orphan
        // blocks, and the script does not fall through to its caller.
        let mut main_terminated = false;

        for stmt in &script.statements {
            // If the current block is already terminated, subsequent
            // statements are dead code.  Route them into a fresh orphan
            // block with no incoming edge (rather than dropping them) so
            // SCCP marks it unreachable and O107 can flag the dead code.
            if self.block_mut(&current).terminator.is_some() {
                main_terminated = true;
                current = self.new_block("unreachable");
            }

            // `break` / `continue` inside a loop body lower to a CFG edge to
            // the loop's exit / continue target, so the loop-exit block stays
            // reachable (a `while 1 { … break }` post-loop block is live).
            if self.lower_loop_jump(&current, stmt) {
                continue;
            }
            current = self.lower_script_statement(stmt, &current)?;
        }

        // Always return the block control finally rests in, even when it is
        // terminated by a straight-line `return`/`error`.
        // `build_function` then appends a synthetic (unreachable) `exit` block
        // via `ensure_goto` (a no-op on the already-terminated block).
        // Termination is tracked separately in
        // `last_terminal_block` so `lower_try`
        // can source an on-error edge from a body that ended without an explicit
        // `error`/`throw`.  Nested control-flow lowerings still signal "no
        // continuation" by returning `None` (propagated through this loop's
        // `?` / explicit-`None` arms).
        let terminated = main_terminated || self.block_mut(&current).terminator.is_some();
        self.last_terminal_block = if terminated {
            Some(current.clone())
        } else {
            None
        };
        Some(current)
    }

    /// Dispatch `Foreach` — dict for/map, opaque top-level, or inlined.
    fn lower_foreach_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Foreach {
            is_dict_iteration,
            is_array_iteration,
            raw_args,
            raw_tokens,
            iterators,
            is_lmap,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        // `array for {k v} arr body` (Tcl 9.0): the body runs in the caller's
        // frame, so the analysis CFG inlines it (shared reaching-defs / const
        // lattice, loop vars bound) while codegen barriers it to the
        // `::tcl::array::for` ensemble invoke the array hook resolves — keeping
        // the emitted bytecode byte-identical to C Tcl.
        if *is_array_iteration {
            if self.faithful_exceptions {
                return self.lower_foreach(stmt, current);
            }
            self.block_mut(current).statements.push(Statement::Barrier {
                span: *span,
                reason: "array for".into(),
                command: "array".into(),
                canonical_command: None,
                args: raw_args.clone(),
                tokens: raw_tokens.clone(),
            });
            return current.to_owned();
        }

        if *is_dict_iteration && !raw_args.is_empty() {
            // `dict for`/`dict map {k v} $d body`: the body runs in the caller's
            // frame, so the analysis CFG inlines it (shared reaching-defs / const
            // lattice, loop vars bound, and — crucially — the body's *own* control
            // flow lowered into real blocks, so a read nested inside an `if` /
            // `while` / `catch` body is a first-class SSA use). Codegen barriers it
            // to the `::tcl::dict::for`/`::tcl::dict::map` ensemble invoke below,
            // keeping the emitted bytecode byte-identical to C Tcl. Mirrors the
            // `array for` split above. Without the inlined body, a command-name or
            // brace-nested `$var` read the shallow barrier-word scan can't see is
            // lost, so the loop's outer reads look dead (issue #833).
            if self.faithful_exceptions {
                return self.lower_foreach(stmt, current);
            }
            let sub = &raw_args[0];
            let qual_cmd = format!("::tcl::dict::{sub}");
            // The barrier re-emits `::tcl::dict::for {vars} $dict {body}` — the
            // source `dict for …` tokens with the `for`/`map` subcommand word
            // dropped — so the braced var-list / body push verbatim and the
            // `$dict` argument substitutes (else the body's command subs are
            // evaluated once, before the loop variables exist).
            let args = raw_args[1..].to_vec();
            let tokens = raw_tokens
                .as_ref()
                .map_or_else(|| all_str_tokens(&qual_cmd, &args), |t| drop_word(t, 1));
            self.block_mut(current).statements.push(Statement::Barrier {
                span: *span,
                reason: "dict for/map".into(),
                command: qual_cmd,
                canonical_command: None,
                args,
                tokens: Some(tokens),
            });
            return current.to_owned();
        }

        let has_qualified_vars = iterators
            .iter()
            .any(|it| it.vars.iter().any(|v| v.starts_with("::")));

        if (!self.inline_loops && !raw_args.is_empty()) || has_qualified_vars {
            let cmd = if *is_lmap { "lmap" } else { "foreach" };
            let loop_vars: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: cmd.into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs: loop_vars,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(frozen_loop_tokens(cmd, raw_args, raw_tokens.as_ref())),
                foreach_groups: None,
            });
            return current.to_owned();
        }

        self.lower_foreach(stmt, current)
    }

    /// Emit an opaque `catch` call with defs for modified variables.
    fn emit_opaque_catch(&mut self, stmt: &Statement, current: &str) {
        let Statement::Catch {
            body,
            result_var,
            options_var,
            raw_args,
            span,
            tokens,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let mut catch_defs = defs_from_ir_script(body);
        if let Some(rv) = result_var {
            catch_defs.push(rv.clone());
        }
        if let Some(ov) = options_var {
            catch_defs.push(ov.clone());
        }
        dedup_preserve_order(&mut catch_defs);
        // Preserve ``tokens`` on the synthetic ``Statement::Call``
        // so the codegen's eval-fallback can detect the braced
        // body word and re-wrap it in ``{…}`` when reconstructing
        // the script for ``tcl_eval``.  Without this,
        // ``catch {$undef} msg`` would lower to ``catch $undef
        // msg`` and the var-read trap would fire before catch
        // could intercept it.
        self.block_mut(current).statements.push(Statement::Call {
            span: *span,
            command: "catch".into(),
            canonical_command: None,
            args: raw_args.clone(),
            defs: catch_defs,
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: tokens.clone(),
            foreach_groups: None,
        });
    }

    /// Dispatch `Try` — deferred opaque or inlined.
    fn lower_try_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Try {
            handlers,
            finally_body,
            raw_args,
            body,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let defer = !self.inline_loops
            && !raw_args.is_empty()
            && (!handlers.is_empty() || finally_body.is_none());

        if defer {
            let mut try_defs = defs_from_ir_script(body);
            for handler in handlers {
                if let Some(vn) = &handler.var_name {
                    try_defs.push(vn.clone());
                }
                if let Some(ov) = &handler.options_var {
                    try_defs.push(ov.clone());
                }
                try_defs.extend(defs_from_ir_script(&handler.body));
            }
            if let Some(fb) = finally_body {
                try_defs.extend(defs_from_ir_script(fb));
            }
            dedup_preserve_order(&mut try_defs);
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: "try".into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs: try_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            });
            return current.to_owned();
        }

        self.lower_try(stmt, current)
    }
}

// Public API

/// Every spelling a call site may use for the procedure named `qname`.
///
/// A qualified definition is reachable by its absolute name, by its bare
/// tail, **and** by any `::`-boundary suffix in between: written from
/// `::other`, the word `demo::setdef` resolves relative to the current
/// namespace first and then falls back to `::demo::setdef`, so that
/// spelling has to key the same summary (issue #923 audit idx 59, where the
/// relative-qualified spelling silently missed the caller-frame defs while
/// the bare and absolute spellings both worked).
///
/// Registering a suffix is the same over-approximation the bare tail
/// already was: a same-named proc in a different namespace can claim the
/// key. The consequence is a *widened* `defs` list at a call site, which
/// only ever silences a warning.
#[must_use]
pub fn qualified_lookup_keys(qname: &str) -> Vec<String> {
    let mut keys = vec![qname.to_owned()];
    let mut push = |key: &str| {
        if !key.is_empty() && !keys.iter().any(|k| k == key) {
            keys.push(key.to_owned());
        }
    };
    // `::demo::setdef` → `demo::setdef` → `setdef`: drop one leading
    // namespace segment at a time, so every relative spelling of the same
    // definition keys the same summary.
    let mut rest = qname.trim_start_matches("::");
    push(rest);
    while let Some((_, tail)) = rest.split_once("::") {
        let tail = tail.trim_start_matches("::");
        push(tail);
        rest = tail;
    }
    keys
}

/// Scan a module for procedures with a caller-frame effect
/// (`upvar` / `uplevel`), returning a map from command name to its
/// [frame-effect summary][UpvarInfo].  Every spelling a call site may use
/// ([`qualified_lookup_keys`]) is registered so the lookup does not depend
/// on how the caller happened to write the name.
#[must_use]
pub fn detect_upvar_procs(module: &Module) -> HashMap<String, UpvarInfo> {
    let mut result: HashMap<String, UpvarInfo> = HashMap::new();
    // Deterministic (qualified-name) iteration order, for the same reason
    // [`prepare_cfg_context`] does it for `proc_params`: the *short* key below is
    // written by every procedure sharing that short name, so last-write-wins must
    // not be decided by the `HashMap`'s random per-process seed — this map is part
    // of the `CfgContext` folded into every procedure's `function_lattice` memo
    // key, and a nondeterministic winner makes the per-procedure cache hit or miss
    // by luck of the process start (issue #1035 follow-up).
    let mut entries: Vec<(&String, &crate::ir::Procedure)> = module.procedures.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut own: Vec<(&String, &crate::ir::Procedure, UpvarInfo)> = Vec::new();
    for (qname, proc) in entries {
        own.push((qname, proc, collect_upvar_targets(&proc.body, &proc.params)));
    }

    // One hop, no fixpoint: `uplevel <caller frame> [list callee …]` puts
    // the callee's own caller-frame effects into *this* proc's caller
    // (issue #1019). Composing against the *own-body* summaries — not the
    // composed ones — bounds the walk at a single level by construction, so
    // a recursive or mutually-recursive forward cannot diverge. A two-hop
    // chain is left to the opaque-widening path, which is the safe
    // direction.
    // `own` is already qualified-name-sorted, so a short key shared by two
    // procedures resolves to the same one every run.
    let mut by_name: HashMap<String, (&UpvarInfo, &[String])> = HashMap::new();
    for (qname, proc, info) in &own {
        for key in qualified_lookup_keys(qname) {
            by_name.insert(key, (info, proc.params.as_slice()));
        }
    }
    let mut composed: Vec<(&String, UpvarInfo)> = Vec::new();
    for (qname, proc, own_info) in &own {
        let mut info = own_info.clone();
        for (callee, constructed) in &own_info.uplevel_forwarded_calls {
            if let Some((callee_info, callee_params)) = by_name.get(callee.as_str()) {
                upvar_info::compose_forwarded(
                    callee_info,
                    callee_params,
                    constructed,
                    &proc.params,
                    &mut info,
                );
            } else {
                // The forwarded command is not a procedure this unit can
                // see (a cross-file helper, a runtime-installed command).
                // It still runs in the caller's frame, so widen.
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        // The second one-hop composition: an **ordinary** call to a proc that
        // reaches past its own caller lands in *this* proc's caller (issue
        // #1019 / issue #923 idx 24).  Oracle, identical on tclsh 9.0.4 and
        // 8.6.16: `proc setUp2 {var} {uplevel 2 [list set $var 99]}` /
        // `proc middle {} {setUp2 answer}` / `proc outer {} {middle; return
        // $answer}` — `outer` returns `99`, so `answer` really is assigned in
        // a frame `middle` never names.  A level-**1** effect is emphatically
        // not transitive this way (`detect_upvar_procs_does_not_propagate_
        // through_a_plain_call_wrapper` pins the tclsh transcript), which is
        // why only a `FrameReach::PastTheCaller` summary travels here.
        //
        // One hop, against the own-body summaries, for the same reason the
        // forwarded-call composition above takes exactly one: it terminates
        // by construction on any call graph, recursive or not.  A `uplevel 3`
        // two plain calls out is left to the opaque-widening path.
        for callee in &own_info.plain_calls {
            if by_name
                .get(callee.as_str())
                .is_some_and(|(callee_info, _)| {
                    callee_info.frame_reach == FrameReach::PastTheCaller
                })
            {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        // Composition input, not a caller-side effect — dropped so the
        // published summaries (which are folded into every procedure's
        // `function_lattice` memo key) carry only what a call site reads.
        info.plain_calls.clear();
        composed.push((qname, info));
    }

    for (qname, info) in composed {
        if info.is_empty() {
            continue;
        }
        for key in qualified_lookup_keys(qname) {
            if key == *qname {
                continue;
            }
            result.insert(key, info.clone());
        }
        result.insert(qname.clone(), info);
    }
    result
}

/// Module-wide CFG-determining context: the upvar-procs map, the
/// parameter-list map, and the global-write-procs map
/// ([`global_write_info::detect_global_write_procs`]) [`prepare_cfg_context`]
/// returns. The single canonical definition [`crate::compilation_unit`]
/// reuses for its own `CfgContext` alias, so the two never drift apart.
pub type CfgContext = (
    HashMap<String, UpvarInfo>,
    HashMap<String, Vec<String>>,
    HashMap<String, GlobalWriteInfo>,
);

/// Return the upvar-procs map, the parameter-list map, and the
/// global-write-procs map ([`global_write_info::detect_global_write_procs`])
/// used by the CFG builder's call-site invalidation pass.  Both the
/// qualified and short forms are registered for every proc.
#[must_use]
pub fn prepare_cfg_context(module: &Module) -> CfgContext {
    let upvar_procs = detect_upvar_procs(module);
    let global_write_procs = global_write_info::detect_global_write_procs(module);
    let mut proc_params: HashMap<String, Vec<String>> = HashMap::new();
    // Iterate procedures in a deterministic (qualified-name) order: a *short*
    // name shared by two procedures (`::a::x` and `::b::x`) is inserted by both,
    // so last-write-wins must be order-independent — otherwise `proc_params`
    // (and the `CfgContext` it feeds into the `function_lattice` memo key) would
    // depend on `HashMap` iteration order and the per-procedure lattice cache
    // would hit or miss by random seed.  Qualified names are unique, so their
    // entries are unaffected.
    let mut entries: Vec<(&String, &crate::ir::Procedure)> = module.procedures.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (qname, proc) in entries {
        for key in qualified_lookup_keys(qname) {
            proc_params.insert(key, proc.params.clone());
        }
    }
    (upvar_procs, proc_params, global_write_procs)
}

/// Build CFGs for a whole module: top-level script + each procedure.
///
/// When `defer_top_level` is `true`, `foreach`/`catch`/`try` at the
/// top level are compiled as opaque calls (matching tclsh bytecode
/// output). Analysis passes should leave this `false` to get full
/// inlining of loop bodies.
///
/// The builder also applies the upvar-invalidation pass — calls to
/// procedures whose bodies use `upvar` have their `defs` augmented
/// with the caller-side variable names the callee will modify.  See
/// [`prepare_cfg_context`] for the per-module scan.
#[must_use]
pub fn build_cfg(module: &Module, defer_top_level: bool) -> CfgModule {
    // The module carries its dialect *name*; resolve it once through the
    // grammar owner so a host or test that builds a CFG straight from a
    // module still lexes its bodies under the module's grammar. A caller
    // holding the document's config uses `build_cfg_with_config`.
    build_cfg_inner(
        module,
        defer_top_level,
        true,
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
            module.dialect.as_deref(),
        )),
    )
}

/// [`build_cfg`] with the document's own [`tcl_lexer::LexerConfig`], threaded
/// into every embedded-`[…]` re-lex the builder performs (see
/// [`CfgBuilder::config`]).
#[must_use]
pub fn build_cfg_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner(module, defer_top_level, true, config)
}

/// Build CFGs for codegen (bytecode / WASM): the plain, byte-identical loop /
/// switch shape with **no** analysis-only transforms (`faithful_exceptions`
/// off).  The terminator promotions (`tailcall`, all-exit opaque switch), the
/// opaque-switch loop-jump edges, and the guaranteed-iteration loop rotation
/// are gated on `faithful_exceptions`, so they appear only in the analysis CFG
/// ([`build_cfg`]) and never in the CFG codegen lowers — keeping the emitted
/// bytecode / CFG shape identical to the unannotated source.
#[must_use]
pub fn build_cfg_codegen(module: &Module, defer_top_level: bool) -> CfgModule {
    // See `build_cfg`: the module's dialect name resolves once through the
    // grammar owner.
    build_cfg_inner(
        module,
        defer_top_level,
        false,
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
            module.dialect.as_deref(),
        )),
    )
}

/// [`build_cfg_codegen`] with the document's own [`tcl_lexer::LexerConfig`].
#[must_use]
pub fn build_cfg_codegen_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner(module, defer_top_level, false, config)
}

fn build_cfg_inner(
    module: &Module,
    defer_top_level: bool,
    faithful: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let (upvar_procs, proc_params, global_write_procs) = prepare_cfg_context(module);

    let new_builder = |inline: bool| {
        let b = CfgBuilder::new_with_upvars(
            inline,
            upvar_procs.clone(),
            proc_params.clone(),
            global_write_procs.clone(),
        )
        .with_lexer_config(config);
        if faithful {
            b.with_faithful_exceptions()
        } else {
            b
        }
    };

    let mut top_builder = new_builder(!defer_top_level);
    let top_cfg = top_builder.build_function("::top", &module.top_level);

    let mut proc_cfgs = HashMap::new();
    for (qname, proc) in &module.procedures {
        let mut builder = new_builder(true);
        proc_cfgs.insert(qname.clone(), builder.build_function(qname, &proc.body));
    }

    CfgModule {
        top_level: top_cfg,
        procedures: proc_cfgs,
    }
}

/// Build a CFG for a single script body.  Does not apply the upvar-
/// invalidation pass — use [`build_cfg`] when a whole module is
/// available and call-site def invalidation matters.
#[must_use]
pub fn build_cfg_function(name: &str, script: &Script, inline_loops: bool) -> Function {
    let mut builder = CfgBuilder::new(inline_loops);
    builder.build_function(name, script)
}

/// Build a CFG for a single script body with an explicit upvar
/// context (from [`prepare_cfg_context`]). Used for `TclOO` method
/// bodies, which are lowered to their own [`Function`]s
/// outside [`build_cfg`] (methods are deliberately excluded from
/// [`CfgModule::procedures`] — codegen never emits them) but still
/// need the same call-site def invalidation as procs.
///
/// The maps usually come straight from [`prepare_cfg_context`] (default
/// hasher); the signature is generalised over `BuildHasher` and rehashes
/// into the builder's default-hashed maps on entry.
#[must_use]
pub fn build_cfg_function_with_upvars<S1, S2, S3>(
    name: &str,
    script: &Script,
    inline_loops: bool,
    upvar_procs: HashMap<String, UpvarInfo, S1>,
    proc_params: HashMap<String, Vec<String>, S2>,
    global_write_procs: HashMap<String, GlobalWriteInfo, S3>,
) -> Function
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    // Rehash into the builder's default-hashed maps (cheap, once per function).
    let upvar_procs: HashMap<String, UpvarInfo> = upvar_procs.into_iter().collect();
    let proc_params: HashMap<String, Vec<String>> = proc_params.into_iter().collect();
    let global_write_procs: HashMap<String, GlobalWriteInfo> =
        global_write_procs.into_iter().collect();
    // dialect-drift-ok: the dialect-blind form; `build_cfg_function_with_upvars_and_config`
    // is what a caller holding the document's config uses.
    let mut builder =
        CfgBuilder::new_with_upvars(inline_loops, upvar_procs, proc_params, global_write_procs)
            .with_faithful_exceptions()
            .with_lexer_config(tcl_lexer::LexerConfig::default()); // dialect-drift-ok: see above
    builder.build_function(name, script)
}

/// [`build_cfg_function_with_upvars`] with the document's own
/// [`tcl_lexer::LexerConfig`].
#[must_use]
pub fn build_cfg_function_with_upvars_and_config<S1, S2, S3>(
    name: &str,
    script: &Script,
    inline_loops: bool,
    upvar_procs: HashMap<String, UpvarInfo, S1>,
    proc_params: HashMap<String, Vec<String>, S2>,
    global_write_procs: HashMap<String, GlobalWriteInfo, S3>,
    config: tcl_lexer::LexerConfig,
) -> Function
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
    S3: std::hash::BuildHasher,
{
    let upvar_procs: HashMap<String, UpvarInfo> = upvar_procs.into_iter().collect();
    let proc_params: HashMap<String, Vec<String>> = proc_params.into_iter().collect();
    let global_write_procs: HashMap<String, GlobalWriteInfo> =
        global_write_procs.into_iter().collect();
    let mut builder =
        CfgBuilder::new_with_upvars(inline_loops, upvar_procs, proc_params, global_write_procs)
            .with_faithful_exceptions()
            .with_lexer_config(config);
    builder.build_function(name, script)
}

/// Deduplicate a `Vec` while preserving first-occurrence order.
fn dedup_preserve_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

// Registry-derived command classifications
//
// The CFG builder's public entry points ([`build_cfg`],
// [`build_cfg_function`]) carry no registry handle, and several
// classification consumers ([`flow_facts_stmt`],
// [`escaping_loop_jumps`]) are free functions shared with SSA — so the
// name sets are materialised once per process from the shared plain-Tcl
// registry ([`registry_for_dialect`]'s cached `&'static` instance)
// instead of being threaded through every signature. The sets replace
// the hardcoded name matches this file used to carry; the drift test
// `registry_derived_cfg_classes_match_previous_hardcodes` pins their
// contents so a registry stamping change is a conscious CFG decision.

/// Name sets for the command classifications the CFG builder keys
/// control-flow shape on, each derived from one registry trait.
struct CfgCommandClasses {
    /// [`Traits::TERMINATES_BLOCK`] minus specs lowered through
    /// [`LoweringHookId::Return`]: `return` carries the trait but
    /// reaches the builder as its own [`Statement::Return`], never as a
    /// plain `Statement::Call`, so it has no business in the
    /// name-keyed terminator set.
    terminates_block: HashSet<&'static str>,
    /// [`Traits::CATCHABLE_THROW`] — raises `TCL_ERROR`, sourcing an
    /// enclosing `try`'s on-error edge (`error` / `throw`; not `exit`,
    /// which kills the process outside any exception range).
    catchable_throw: HashSet<&'static str>,
    /// [`Traits::REPLACES_FRAME`] — replaces the procedure frame and
    /// never returns to the calling body (`tailcall`).
    replaces_frame: HashSet<&'static str>,
    /// [`Traits::BREAKS_LOOP`] — jumps to the enclosing loop's
    /// post-loop target.
    breaks_loop: HashSet<&'static str>,
    /// [`Traits::CONTINUES_LOOP`] — jumps to the enclosing loop's
    /// next-iteration target.
    continues_loop: HashSet<&'static str>,
}

/// The classification sets, built once from the cached plain-Tcl
/// registry. All five classes are core-Tcl commands present in every
/// dialect, so the plain registry is the right (and dialect-stable)
/// source — exactly the set the retired hardcoded matches encoded.
static CFG_COMMAND_CLASSES: LazyLock<CfgCommandClasses> = LazyLock::new(|| {
    let registry = static_context_for("").commands();
    let with = |t: Traits| -> HashSet<&'static str> {
        registry.commands_with_trait(t).into_iter().collect()
    };
    let terminates_block = registry
        .commands_with_trait(Traits::TERMINATES_BLOCK)
        .into_iter()
        .filter(|name| {
            registry
                .get(name)
                .is_none_or(|s| s.lowering_hook != Some(LoweringHookId::Return))
        })
        .collect();
    CfgCommandClasses {
        terminates_block,
        catchable_throw: with(Traits::CATCHABLE_THROW),
        replaces_frame: with(Traits::REPLACES_FRAME),
        breaks_loop: with(Traits::BREAKS_LOOP),
        continues_loop: with(Traits::CONTINUES_LOOP),
    }
});

/// Builtin commands that unconditionally terminate the current block —
/// the registry's `Traits::TERMINATES_BLOCK` set (minus `return`, which
/// is handled separately as its own `Statement::Return`; see
/// [`CfgCommandClasses::terminates_block`]).  Leading `:` runs are
/// trimmed so a qualified `::error` classifies like `error`, as the
/// retired hardcoded match did.
fn is_block_terminating_command(command: &str) -> bool {
    CFG_COMMAND_CLASSES
        .terminates_block
        .contains(command.trim_start_matches(':'))
}

/// Whether `command` replaces the current procedure's frame — the
/// registry's `Traits::REPLACES_FRAME` set (`tailcall`, canonical
/// `::tailcall`).
///
/// `tailcall` (Tcl 8.6+) replaces the current procedure's frame and never
/// returns here: `TclNRTailcallObjCmd` (generic/tclBasic.c) always
/// `return TCL_RETURN` — both bare `tailcall` and `tailcall command ...` exit
/// the proc; the arg count only decides what runs *after* the frame pops, not
/// whether this proc continues.  So *any* frame-replacing command ends
/// straight-line flow exactly like `error` / `exit` (with no args guard).
fn is_tailcall_command(command: &str) -> bool {
    CFG_COMMAND_CLASSES
        .replaces_frame
        .contains(command.trim_start_matches(':'))
}

/// Whether `name` (pre-normalised by the caller — raw at
/// [`CfgBuilder::lower_loop_jump`], `:`-trimmed elsewhere, matching
/// what each site historically compared) jumps to the enclosing loop's
/// post-loop target — the registry's `Traits::BREAKS_LOOP` set.
fn is_loop_break_command(name: &str) -> bool {
    CFG_COMMAND_CLASSES.breaks_loop.contains(name)
}

/// Loop-jump twin of [`is_loop_break_command`] for the next-iteration
/// target — the registry's `Traits::CONTINUES_LOOP` set.
fn is_loop_continue_command(name: &str) -> bool {
    CFG_COMMAND_CLASSES.continues_loop.contains(name)
}

// Definite-assignment ("flow facts") over un-lowered IR scripts
//
// Shared by the CFG builder (to promote an opaque `switch` whose every arm
// exits the *procedure* — FP-RBS-15 — and to wire its loop-jump edges) and
// by SSA (to recover the def set an opaque switch *definitely* establishes —
// FP-RBS-14).  Both consumers agree on one model.
//
// Each statement/script is classified by how it leaves: it can complete NORMALly
// (fall through to the next statement), jump within an enclosing loop (LoopJump
// — `break`/`continue`), or exit the procedure (ProcExit —
// `return`/`error`/`throw`/`exit`/`tailcall`).  The distinction matters for an
// *opaque* switch (whose arm bodies are not lowered into the CFG, so their
// loop-jump edges are invisible): a ProcExit arm reaches no later code at all,
// but a LoopJump arm still reaches the code after the enclosing loop *without*
// the other arms' definitions — so it must NOT be treated as vacuous when
// recovering the switch's defs, and an all-LoopJump switch is NOT a procedure
// terminator.

/// How a statement/script leaves: fall through, jump to an enclosing loop, or
/// exit the procedure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Completion {
    /// Falls through to the next statement.
    Normal,
    /// `break` / `continue` — leaves to an enclosing-loop target (still reaches
    /// the code after that loop).
    LoopJump,
    /// `return` / `error` / `throw` / `exit` / `tailcall` — leaves the proc.
    ProcExit,
}

/// `(must-defines, completion)` for a single statement.
///
/// * assignments (`set`/`incr`/`expr`-assign) contribute their target and
///   complete normally; a plain `Call` contributes its synthetic `defs` and
///   completes normally — *except* `error`/`throw`/`exit` and `tailcall`, which
///   `ProcExit`, and `break`/`continue`, which `LoopJump`; `Return` and a
///   `return -code` / expansion `Barrier` `ProcExit`;
/// * `Block` / `UpFrame` bodies always run, so recurse;
/// * an `If` *with* an else, or a `Switch` *with* a default, combine their
///   branches via [`intersect_completing`]; an else-less `If` / default-less
///   `Switch` has a fall-through path, so it completes normally assigning
///   nothing for certain;
/// * loops (`For`/`While`/`Foreach`), `Catch`/`Try` and everything else may not
///   execute / always complete (`catch` even swallows `break`), so they
///   contribute no must-define and complete normally.
pub(crate) fn flow_facts_stmt(stmt: &Statement) -> (BTreeSet<String>, Completion) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => {
            let mut set = BTreeSet::new();
            let n = normalise_var_name(name);
            if !n.is_empty() {
                set.insert(n.to_owned());
            }
            (set, Completion::Normal)
        }
        Statement::Return { .. } => (BTreeSet::new(), Completion::ProcExit),
        Statement::Barrier {
            reason,
            command,
            canonical_command,
            ..
        } => {
            // A `return -options …` / `return {*}…` barrier unconditionally
            // exits the proc.  An opaque callback barrier still retains its
            // host command identity; preserve registry-declared unconditional
            // control transfer such as `tailcall` rather than treating the
            // callback's unknown effects as evidence that the host falls
            // through.
            let canon = canonical_command.as_deref().unwrap_or(command);
            if matches!(
                reason.as_str(),
                "return with options" | "return with expansion"
            ) || is_block_terminating_command(canon)
                || is_tailcall_command(canon)
            {
                (BTreeSet::new(), Completion::ProcExit)
            } else {
                (BTreeSet::new(), Completion::Normal)
            }
        }
        Statement::Call {
            command,
            canonical_command,
            defs,
            ..
        } => {
            let canon = canonical_command.as_deref().unwrap_or(command);
            let bare = canon.trim_start_matches(':');
            let completion = if is_loop_break_command(bare) || is_loop_continue_command(bare) {
                // A loop jump leaves to the enclosing loop's target — it still
                // reaches the code after that loop, just without later defs.
                Completion::LoopJump
            } else if is_block_terminating_command(canon) || is_tailcall_command(canon) {
                Completion::ProcExit
            } else {
                Completion::Normal
            };
            (defs.iter().cloned().collect(), completion)
        }
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => flow_facts_script(body),
        Statement::If {
            clauses,
            else_body: Some(eb),
            ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            bodies.push(eb);
            intersect_completing(&bodies)
        }
        Statement::Switch {
            arms,
            default_body: Some(default),
            ..
        } => {
            let mut bodies: Vec<&Script> = vec![default];
            bodies.extend(arms.iter().filter_map(|arm| arm.body.as_ref()));
            intersect_completing(&bodies)
        }
        // An else-less `If` / default-less `Switch` has a fall-through path that
        // assigns nothing for certain; everything else may not run / always
        // completes.
        _ => (BTreeSet::new(), Completion::Normal),
    }
}

/// `(vars definitely assigned, completion)` for an un-lowered IR script.
///
/// Walk statements in order accumulating must-defines; the first statement that
/// does not complete normally makes the rest dead and gives the script that
/// statement's completion (`ProcExit` or `LoopJump`), with the must-defines
/// being those accumulated up to (and including) it.  Only names guaranteed to
/// be assigned are returned, so it never over-claims (which would hide a real
/// read-before-set).
pub(crate) fn flow_facts_script(script: &Script) -> (BTreeSet<String>, Completion) {
    let mut assigned = BTreeSet::new();
    for stmt in &script.statements {
        let (defs, completion) = flow_facts_stmt(stmt);
        assigned.extend(defs);
        if completion != Completion::Normal {
            return (assigned, completion);
        }
    }
    (assigned, Completion::Normal)
}

/// Combine branch bodies into `(must-defines, completion)`.
///
/// A `ProcExit` branch reaches no code after the construct, so it is vacuous
/// (⊤) and excluded from the must-define intersection.  A `Normal` or
/// `LoopJump` branch *does* reach later code (the fall-through successor, or —
/// for a loop jump — the code after the enclosing loop), so its must-defines
/// (those established before it leaves) ARE intersected: this keeps the result
/// sound when a `break`/`continue` arm escapes an opaque switch without the
/// other arms' defs.  The combined completion is `Normal` if any branch falls
/// through, else `LoopJump` if any jumps, else `ProcExit` (every branch exits).
fn intersect_completing(bodies: &[&Script]) -> (BTreeSet<String>, Completion) {
    let mut common: Option<BTreeSet<String>> = None;
    let mut any_normal = false;
    let mut any_loop_jump = false;
    for body in bodies {
        let (assigned, completion) = flow_facts_script(body);
        match completion {
            Completion::ProcExit => continue,
            Completion::Normal => any_normal = true,
            Completion::LoopJump => any_loop_jump = true,
        }
        common = Some(match common {
            None => assigned,
            Some(acc) => acc.intersection(&assigned).cloned().collect(),
        });
    }
    let combined = if any_normal {
        Completion::Normal
    } else if any_loop_jump {
        Completion::LoopJump
    } else {
        Completion::ProcExit
    };
    (common.unwrap_or_default(), combined)
}

/// Variables an opaque `switch` assigns on *every* non-proc-exit path.
///
/// Empty unless the switch has a `default` arm (otherwise an unmatched subject
/// falls through assigning nothing).  Intersection over the default and every
/// arm-with-a-body, excluding only arms that exit the procedure (which reach no
/// later code).  A `break`/`continue` arm is *kept* (with the defs it makes
/// before jumping), because it still reaches the code after the enclosing loop —
/// so an arm that breaks without assigning `y` correctly drops `y` rather than
/// letting it be claimed defined.
pub(crate) fn switch_must_defines(stmt: &Statement) -> BTreeSet<String> {
    flow_facts_stmt(stmt).0
}

/// `(can_break, can_continue)` for `break`/`continue` that escape *script* to an
/// enclosing loop.
///
/// Recurses into `if`/`switch`/`Block`/`UpFrame` bodies (which don't capture a
/// loop jump) and into `try` — a `try` does NOT capture `break`/`continue`:
/// without a matching `on`/`trap` handler a jump in the body, a handler body, or
/// the `finally` body propagates to the enclosing loop (confirmed in tclsh
/// 9.0.3, contrast `catch {break}` which yields code 3 and is absorbed).  Does
/// NOT recurse into nested loops (`for`/`while`/`foreach`) or `catch`, which
/// capture their own jumps.  Scanning stops at the first statement that cannot
/// complete normally (`return`/`error`/`exit`/`tailcall`, or `break`/`continue`
/// itself): a jump after it is dead code that never executes, so it must not
/// create a spurious loop-exit edge.
/// `depth` is the nesting level of `script` — reuses [`MAX_LOWER_DEPTH`]
/// (this walk is mutually recursive with [`switch_escaping_jumps`], and
/// both are transitively bounded today via the same cap `lower_script`
/// already builds every `Script` under, so this is defence-in-depth /
/// consistency with that cap rather than a currently-reachable path).
pub(crate) fn escaping_loop_jumps(script: &Script, depth: u32) -> (bool, bool) {
    if MAX_LOWER_DEPTH.exceeded(depth) {
        return (false, false);
    }
    let mut can_break = false;
    let mut can_continue = false;
    for stmt in &script.statements {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                ..
            } => {
                let bare = canonical_command
                    .as_deref()
                    .unwrap_or(command)
                    .trim_start_matches(':');
                if is_loop_break_command(bare) {
                    can_break = true;
                } else if is_loop_continue_command(bare) {
                    can_continue = true;
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    let (b, c) = escaping_loop_jumps(&clause.body, depth + 1);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(eb) = else_body {
                    let (b, c) = escaping_loop_jumps(eb, depth + 1);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            Statement::Switch { .. } => {
                let (b, c) = switch_escaping_jumps(stmt, depth);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                let (b, c) = escaping_loop_jumps(body, depth + 1);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                let (b, c) = escaping_loop_jumps(body, depth + 1);
                can_break |= b;
                can_continue |= c;
                for h in handlers {
                    let (b, c) = escaping_loop_jumps(&h.body, depth + 1);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(fb) = finally_body {
                    let (b, c) = escaping_loop_jumps(fb, depth + 1);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            _ => {}
        }
        // A statement that cannot complete normally makes everything after it
        // dead code (a later `break`/`continue` never runs), so stop scanning —
        // otherwise a dead `break` after `error`/`return` would forge a
        // loop-exit edge and fire W210 on unreachable post-loop code.
        if flow_facts_stmt(stmt).1 != Completion::Normal {
            break;
        }
    }
    (can_break, can_continue)
}

/// `(can_break, can_continue)` over all bodies of an opaque `switch`.
/// `depth` is `stmt`'s own nesting level — see [`escaping_loop_jumps`].
pub(crate) fn switch_escaping_jumps(stmt: &Statement, depth: u32) -> (bool, bool) {
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return (false, false);
    };
    let mut can_break = false;
    let mut can_continue = false;
    if let Some(default) = default_body {
        let (b, c) = escaping_loop_jumps(default, depth + 1);
        can_break |= b;
        can_continue |= c;
    }
    for arm in arms {
        if let Some(body) = &arm.body {
            let (b, c) = escaping_loop_jumps(body, depth + 1);
            can_break |= b;
            can_continue |= c;
        }
    }
    (can_break, can_continue)
}

// Guaranteed-iteration loops
//
// A loop whose body provably runs at least once does not skip its body, so a
// variable the body assigns is defined when code after the loop reads it.  The
// usual CFG models every loop as possibly running zero times (the header's exit
// edge), false-firing W210 on such a read.  In analysis builds we *rotate* a
// provably-non-empty loop so the 0-iteration skip becomes a *separate*
// entry-guard edge whose condition is statically true; SCCP marks that edge dead
// and the FP-RBS-16 dead-edge phi filter then ignores the version-0 operand it
// carried — with no synthetic def, so SCCP values are untouched.  `break` /
// `continue` stay real CFG edges, so partial-def exits remain sound.

/// True when `text` is a static, non-empty Tcl list literal.
///
/// Conservative: any `$` / `[` (a possible substitution) disqualifies it, so a
/// runtime-computed list is never claimed non-empty.  `foreach` stores a braced
/// list with its outer braces stripped (`{1 2 3}` → `1 2 3`), so a literal
/// splits directly.
fn list_literal_nonempty(text: &str) -> bool {
    if text.is_empty() || text.contains('$') || text.contains('[') {
        return false;
    }
    !crate::tcl_expr_eval::split_tcl_list(text).is_empty()
}

/// True when a `foreach`/`lmap` provably iterates ≥1 time.
///
/// `foreach` runs `max` over its iterator groups, so *any* non-empty iterator
/// list guarantees at least one iteration (shorter lists just pad their loop
/// vars with `""`).
pub(crate) fn foreach_runs_at_least_once(stmt: &Statement) -> bool {
    let Statement::Foreach { iterators, .. } = stmt else {
        return false;
    };
    iterators
        .iter()
        .any(|it| list_literal_nonempty(&it.list_arg))
}

impl CfgBuilder {
    /// Names a non-`AssignConst` init statement may write.  `None` means "can't
    /// tell" — the caller then drops *all* constant bindings (a write it can't
    /// characterise might clobber any of them).
    ///
    /// A `Call`'s write set includes the caller-side variables an `upvar`-using
    /// callee modifies (direct call or `[upvar_proc …]` embedded substitution),
    /// recovered via [`Self::apply_upvar_invalidation`] — the module upvar pass
    /// adds these to a call's `defs`, but they are absent from the raw IR the
    /// init clause carries.  Without this, `for {set i 0; setter} {$i < 3} …`
    /// where `setter` runs `upvar 1 i i; set i 5` would keep the stale
    /// `i = 0` binding and be wrongly judged guaranteed.
    fn init_written_names(&self, stmt: &Statement) -> Option<Vec<String>> {
        let mut names: Vec<String> = match stmt {
            Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => {
                let n = normalise_var_name(name);
                if n.is_empty() {
                    Vec::new()
                } else {
                    vec![n.to_owned()]
                }
            }
            // A plain call contributes only the (augmented) defs collected
            // below.
            Statement::Call { .. } => Vec::new(),
            // Any other statement shape in the init (a nested if/loop/…) is not
            // characterised here — clear every constant binding to stay sound.
            _ => return None,
        };
        for s in self.upvar_invalidated(stmt.clone()) {
            // A widening barrier means the callee can write ANY caller
            // variable (an `upvar` with a dynamic caller-side name) — no
            // per-name set exists, so report "can't tell" and let the
            // caller drop every constant binding.
            if matches!(s, Statement::Barrier { .. }) {
                return None;
            }
            if let Statement::Call { defs, .. } = &s {
                for d in defs {
                    if !names.contains(d) {
                        names.push(d.clone());
                    }
                }
            }
        }
        Some(names)
    }

    /// True when a `for` loop's condition is statically true on entry.
    ///
    /// Evaluates the condition against the constant bindings the init clause
    /// establishes (`for {set i 0} {$i < 3} …` → `i = 0` → `0 < 3` is true),
    /// which proves the first iteration always runs.  Init statements are
    /// processed in order: an `AssignConst` (re)binds a constant, but any
    /// *other* write *invalidates* that variable's binding — so
    /// `for {set i 0; set i $n} …` and `for {set i 0; incr i 5} …` leave `i`
    /// unknown rather than stale-constant `0`, and an `upvar`-writing call in
    /// the init invalidates the var it writes through the caller frame.
    /// Conservative: a condition referencing an unbound variable (or
    /// a command-substitution condition) evaluates to `None` → not guaranteed.
    pub(crate) fn for_runs_at_least_once(&self, stmt: &Statement) -> bool {
        use crate::tcl_expr_eval::{TclValue, eval_tcl_expr};
        let Statement::For {
            init, condition, ..
        } = stmt
        else {
            return false;
        };
        let mut env: crate::tcl_expr_eval::Env = std::collections::HashMap::new();
        for s in &init.statements {
            if let Statement::AssignConst { name, value, .. } = s {
                let n = normalise_var_name(name);
                if !n.is_empty() {
                    env.insert(n.to_owned(), coerce_scalar(value));
                }
                continue;
            }
            match self.init_written_names(s) {
                // Unknown write — every prior constant binding is now suspect.
                None => env.clear(),
                Some(names) => {
                    for n in names {
                        env.remove(&n);
                    }
                }
            }
        }
        match eval_tcl_expr(condition, &env) {
            Some(TclValue::Int(i)) => i != 0,
            Some(TclValue::Float(f)) => f != 0.0,
            // A bignum is canonical (beyond i64), hence never zero.
            Some(TclValue::Big(_)) => true,
            None => false,
        }
    }
}

/// Coerce a literal assignment value to int/float when possible (else str),
/// for the `for`-init constant environment.
fn coerce_scalar(text: &str) -> crate::tcl_expr_eval::EnvValue {
    use crate::tcl_expr_eval::EnvValue;
    if let Ok(i) = text.parse::<i64>() {
        return EnvValue::Int(i);
    }
    if let Ok(f) = text.parse::<f64>() {
        return EnvValue::Float(f);
    }
    EnvValue::Str(text.to_owned())
}

/// Whether `command` raises a *catchable* exception — the registry's
/// `Traits::CATCHABLE_THROW` set (`error` / `throw`); `exit` terminates
/// the process and is not caught by `try`. Throw points of this kind
/// source an enclosing `try`'s on-error edge.
fn is_catchable_throw(command: &str) -> bool {
    CFG_COMMAND_CLASSES
        .catchable_throw
        .contains(command.trim_start_matches(':'))
}

/// Lex *text* into Tcl words, accumulating contiguous tokens between
/// `Sep` / `Eol` separators into single-string words.  `Var` tokens
/// are re-prefixed with `$` so the caller can normalise them via
/// [`crate::naming::normalise_var_name`].
///
/// Returns an empty list when the text fails to lex.
pub(super) fn words_from_text(text: &str) -> Vec<String> {
    // dialect-drift-ok: compatibility shim for the one call site outside this
    // lane (`cfg_builder::upvar_info`); every site here uses the
    // config-carrying form below.
    words_from_text_with_config(text, tcl_lexer::LexerConfig::default())
}

/// [`words_from_text`] under the document's own [`tcl_lexer::LexerConfig`] —
/// where the word boundaries fall is a dialect fact (`}{` under iRules,
/// `$(…)` under Jim, `{*}` from 8.5 on).
pub(super) fn words_from_text_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<String> {
    use tcl_lexer::{Lexer, SourceMap, TokenType};

    let sm = SourceMap::new(text);
    let lexer = Lexer::with_config(text, config);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut words: Vec<String> = Vec::new();
    let mut current: String = String::new();
    let mut prev_sep = true;

    for tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Comment | TokenType::Eof => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
                prev_sep = true;
            }
            _ => {
                let t = sm.token_text(*tok);
                // Re-prepend `$` for `Var` tokens (the lexer strips
                // it on read) so the param-target resolver sees the
                // original `$arg` shape and `normalise_var_name`
                // strips it cleanly.
                let sigil = if matches!(tok.kind, TokenType::Var) {
                    "$"
                } else {
                    ""
                };
                if prev_sep {
                    current.clear();
                }
                current.push_str(sigil);
                current.push_str(t);
                prev_sep = false;
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ForeachIterator, IfClause, Script, SwitchArm, SwitchMode};
    use tcl_lexer::Span;

    /// The registry-derived commands which leave a procedure when they appear
    /// as ordinary calls. Keep test data tied to traits rather than duplicating
    /// command names alongside the production lookup.
    fn proc_exit_host_commands() -> Vec<&'static str> {
        let mut commands: Vec<_> = CFG_COMMAND_CLASSES
            .terminates_block
            .iter()
            .chain(CFG_COMMAND_CLASSES.replaces_frame.iter())
            .copied()
            .collect();
        commands.sort_unstable();
        commands.dedup();
        commands
    }

    fn host_call(command: String) -> Statement {
        Statement::Call {
            span: Span::new(0, 1),
            command,
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    fn host_barrier(command: String) -> Statement {
        Statement::Barrier {
            span: Span::new(0, 1),
            reason: "opaque host command".into(),
            command,
            canonical_command: None,
            args: Vec::new(),
            tokens: None,
        }
    }

    fn build_analysis_cfg(name: &str, script: &Script) -> Function {
        CfgBuilder::new(true)
            .with_faithful_exceptions()
            .build_function(name, script)
    }

    fn proc_exit_barrier_switch(mode: SwitchMode) -> Script {
        let commands = proc_exit_host_commands();
        let arms = commands
            .iter()
            .enumerate()
            .map(|(i, command)| SwitchArm {
                pattern: i.to_string(),
                pattern_span: Span::new(0, 1),
                // Exercise absolute spellings inside both flattened and opaque
                // switch paths; classification must normalise the `::` prefix.
                body: Some(Script::from_statements(vec![host_barrier(format!(
                    "::{command}"
                ))])),
                body_span: Some(Span::new(0, 1)),
                fallthrough: false,
            })
            .collect();
        Script::from_statements(vec![Statement::Switch {
            span: Span::new(0, 1),
            subject: "$which".into(),
            subject_span: Span::new(0, 1),
            arms,
            default_body: Some(Script::from_statements(vec![host_barrier(format!(
                "::{}",
                commands[0]
            ))])),
            default_span: Some(Span::new(0, 1)),
            mode,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        }])
    }

    /// Drift guard: the registry-derived classification sets must equal
    /// the name lists this file used to hardcode, so a future trait
    /// stamping change is a conscious CFG-shape decision rather than a
    /// silent one.
    #[test]
    fn registry_derived_cfg_classes_match_previous_hardcodes() {
        let sorted = |set: &HashSet<&'static str>| -> Vec<&'static str> {
            let mut v: Vec<&'static str> = set.iter().copied().collect();
            v.sort_unstable();
            v
        };
        let classes = &*CFG_COMMAND_CLASSES;
        // `return` carries TERMINATES_BLOCK but lowers to
        // `Statement::Return`, so the name-keyed terminator set excludes it.
        assert_eq!(
            sorted(&classes.terminates_block),
            ["error", "exit", "throw"]
        );
        assert_eq!(sorted(&classes.catchable_throw), ["error", "throw"]);
        assert_eq!(sorted(&classes.replaces_frame), ["tailcall"]);
        assert_eq!(sorted(&classes.breaks_loop), ["break"]);
        assert_eq!(sorted(&classes.continues_loop), ["continue"]);
    }

    /// The classification helpers keep each site's historical name
    /// normalisation: the terminator / throw / tailcall checks trim
    /// leading `:` runs (so canonical `::error` classifies), while
    /// [`CfgBuilder::lower_loop_jump`] matches the raw word (so
    /// `::break` stays a plain call there).
    #[test]
    fn cfg_class_helpers_keep_site_normalisation() {
        assert!(is_block_terminating_command("error"));
        assert!(is_block_terminating_command("::throw"));
        assert!(is_block_terminating_command("exit"));
        assert!(!is_block_terminating_command("return"));
        assert!(!is_block_terminating_command("break"));
        assert!(is_catchable_throw("::error"));
        assert!(!is_catchable_throw("exit"));
        assert!(is_tailcall_command("::tailcall"));
        assert!(!is_tailcall_command("error"));
        assert!(is_loop_break_command("break"));
        assert!(!is_loop_break_command("::break"));
        assert!(is_loop_continue_command("continue"));
        assert!(!is_loop_continue_command("break"));
    }

    #[test]
    fn opaque_host_barriers_match_calls_for_every_registry_proc_exit_command() {
        // This is deliberately registry-driven: today the traits yield
        // `error`, `exit`, `throw`, and `tailcall`, but a trait change must
        // extend this test automatically. Test both source spellings the CFG
        // accepts, including the global-qualified form.
        for command in proc_exit_host_commands() {
            for spelling in [command.to_owned(), format!("::{command}")] {
                for (kind, statement) in [
                    ("Call", host_call(spelling.clone())),
                    ("Barrier", host_barrier(spelling.clone())),
                ] {
                    assert_eq!(
                        flow_facts_stmt(&statement).1,
                        Completion::ProcExit,
                        "{kind} {spelling} must leave the procedure",
                    );

                    let script = Script::from_statements(vec![
                        statement,
                        Statement::AssignConst {
                            span: Span::new(2, 3),
                            name: "dead".into(),
                            name_braced: false,
                            value: "1".into(),
                            value_span: None,
                        },
                    ]);
                    let func = build_analysis_cfg("::test", &script);
                    let entry = &func.blocks[&func.entry];
                    assert_eq!(
                        entry.statements.len(),
                        1,
                        "{kind} {spelling} must keep following statements unreachable",
                    );
                    assert!(
                        matches!(entry.terminator, Some(Terminator::Return { .. })),
                        "{kind} {spelling} must terminate its CFG block, got {:?}",
                        entry.terminator,
                    );
                }
            }
        }
    }

    #[test]
    fn proc_exit_host_barriers_terminate_direct_and_opaque_switch_forms() {
        // Exact switches lower each arm directly; glob switches remain an
        // opaque Statement::Switch. In both representations, barriers whose
        // host command is a registry-described proc exit must retain the same
        // completion fact as a Call.
        let commands = proc_exit_host_commands();
        let direct = build_analysis_cfg("::direct", &proc_exit_barrier_switch(SwitchMode::Exact));
        assert!(matches!(
            direct.blocks[&direct.entry].terminator,
            Some(Terminator::Branch { .. })
        ));
        let direct_barriers: Vec<_> = direct
            .blocks
            .values()
            .filter(|block| {
                block
                    .statements
                    .iter()
                    .any(|stmt| matches!(stmt, Statement::Barrier { .. }))
            })
            .collect();
        assert_eq!(direct_barriers.len(), commands.len() + 1);
        assert!(
            direct_barriers
                .iter()
                .all(|block| { matches!(block.terminator, Some(Terminator::Return { .. })) })
        );

        let opaque = build_analysis_cfg("::opaque", &proc_exit_barrier_switch(SwitchMode::Glob));
        let entry = &opaque.blocks[&opaque.entry];
        assert!(matches!(
            entry.statements.as_slice(),
            [Statement::Switch { .. }]
        ));
        assert!(
            matches!(entry.terminator, Some(Terminator::Return { .. })),
            "an all-proc-exit opaque switch must not fall through, got {:?}",
            entry.terminator,
        );
    }

    #[test]
    fn empty_script_produces_entry_exit() {
        let func = build_cfg_function("::test", &Script::new(), true);
        assert_eq!(func.name, "::test");
        assert!(func.blocks.contains_key(&func.entry));
        // entry → exit
        assert!(func.blocks.len() >= 2);
    }

    #[test]
    fn linear_script() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::AssignConst {
                span: Span::new(8, 15),
                name: "y".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            },
        ]);
        let func = build_cfg_function("::test", &script, true);
        // Entry block should have both statements.
        let entry = &func.blocks[&func.entry];
        assert_eq!(entry.statements.len(), 2);
    }

    #[test]
    fn return_terminates() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::Return {
                span: Span::new(8, 16),
                value: Some("$x".into()),
                expr: None,
                braced: false,
            },
            Statement::AssignConst {
                span: Span::new(17, 24),
                name: "y".into(),
                name_braced: false,
                value: "2".into(), // dead code
                value_span: None,
            },
        ]);
        let func = build_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Only one statement before the return terminator.
        assert_eq!(entry.statements.len(), 1);
        assert!(matches!(entry.terminator, Some(Terminator::Return { .. })));
    }

    #[test]
    fn if_creates_branches() {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 30),
            clauses: vec![IfClause {
                condition: ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 2,
                },
                condition_span: Span::new(3, 5),
                body: Script::from_statements(vec![Statement::AssignConst {
                    span: Span::new(7, 14),
                    name: "y".into(),
                    name_braced: false,
                    value: "1".into(),
                    value_span: None,
                }]),
                body_span: Span::new(6, 15),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
        let func = build_cfg_function("::test", &script, true);
        // Should have at least: entry, if_then, if_next, if_end, exit
        assert!(func.blocks.len() >= 4);
        // Entry block should have a Branch terminator.
        let entry = &func.blocks[&func.entry];
        assert!(
            matches!(entry.terminator, Some(Terminator::Branch { .. })),
            "entry should branch; got {:?}",
            entry.terminator
        );
    }

    #[test]
    fn build_cfg_module() {
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn drop_word_keeps_structured_words_aligned() {
        let tokens = CommandTokens::from_lossy_parts(
            vec![Span::new(0, 4), Span::new(5, 8), Span::new(9, 13)],
            vec!["dict".into(), "for".into(), "body".into()],
            vec![TokenType::Esc, TokenType::Esc, TokenType::Str],
            vec![true, true, true],
            Vec::new(),
            None,
        );
        let dropped = drop_word(&tokens, 1);
        assert_eq!(dropped.argv_texts, vec!["dict", "body"]);
        assert!(dropped.words_align_with_argv_text());
        assert_eq!(dropped.words()[1].legacy_text(), "body");
    }

    #[test]
    fn catch_emits_opaque_call() {
        let script = Script::from_statements(vec![Statement::Catch {
            span: Span::new(0, 30),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "inner".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec!["{set inner 1}".into(), "result".into()],
            tokens: None,
        }]);
        let func = build_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Should have a Call to "catch" with defs.
        assert!(entry.statements.iter().any(|s| matches!(
            s,
            Statement::Call {
                command, defs, ..
            } if command == "catch" && !defs.is_empty()
        )));
    }

    #[test]
    fn foreach_synthetic_call_records_iterator_group_sizes() {
        // ``foreach {a b} L1 c L2 { … }`` has two iterator groups
        // — the first binds two vars, the second binds one.  The
        // synthesised header `Statement::Call` must record the
        // group sizes via ``foreach_groups`` so codegen can
        // reconstruct the original pairing.
        let body = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 0),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]);
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 0),
            iterators: vec![
                ForeachIterator {
                    vars: vec!["a".into(), "b".into()],
                    list_arg: "L1".into(),
                    list_braced: false,
                },
                ForeachIterator {
                    vars: vec!["c".into()],
                    list_arg: "L2".into(),
                    list_braced: false,
                },
            ],
            body,
            body_span: Span::new(0, 0),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);
        let func = build_cfg_function("::test", &script, true);
        let mut found_groups: Option<Vec<usize>> = None;
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command,
                    foreach_groups,
                    ..
                } = stmt
                    && command == "foreach"
                {
                    found_groups = foreach_groups.clone();
                }
            }
        }
        assert_eq!(found_groups, Some(vec![2, 1]));
    }

    #[test]
    fn dedup_preserves_order() {
        let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
        dedup_preserve_order(&mut v);
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    // Uplevel-literal-set wiring tests.
    //
    // Each test drives the full pipeline:
    // `lower_to_ir` → `build_cfg` (which calls `prepare_cfg_context`).
    // The assertions inspect the resulting CFG to confirm that calls
    // to upvar-using procs carry the expected caller-side defs.

    fn lower_module(src: &str) -> Module {
        use tcl_registry::CommandRegistry;
        crate::lowering::lower_to_ir(src, &CommandRegistry::build_default())
    }

    // --- escaping_loop_jumps: try propagation and
    // dead-code early-stop
    //
    // These exercise the helper directly: their end-to-end W210 effect is
    // masked by a separate, pre-existing `while 1` exit-reachability behaviour
    // (Rust treats the post-loop block of an infinite loop as reachable), so we
    // assert the `(can_break, can_continue)` result the wiring keys off.

    #[test]
    fn escaping_loop_jumps_follows_try_but_not_catch() {
        // `try { break }` (no matching handler) propagates the break to the
        // enclosing loop; `catch { break }` absorbs it (yields code 3). Confirmed
        // against tclsh 9.0.3.
        assert_eq!(
            escaping_loop_jumps(&lower_module("try { break }").top_level, 0),
            (true, false),
            "try propagates break",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("try { continue } finally { }").top_level, 0),
            (false, true),
            "try (with finally) propagates continue",
        );
        assert_eq!(
            escaping_loop_jumps(
                &lower_module("try { puts x } on error {} { break }").top_level,
                0
            ),
            (true, false),
            "a break in a try handler body propagates",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("catch { break }").top_level, 0),
            (false, false),
            "catch absorbs break (does not propagate)",
        );
    }

    #[test]
    fn escaping_loop_jumps_stops_after_non_completing_stmt() {
        // A `break`/`continue` after a statement that cannot complete normally
        // is dead code and must not be collected as an escaping jump (else it
        // forges a spurious loop-exit edge).
        assert_eq!(
            escaping_loop_jumps(&lower_module("error bad\nbreak").top_level, 0),
            (false, false),
            "dead break after error is not collected",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("return\ncontinue").top_level, 0),
            (false, false),
            "dead continue after return is not collected",
        );
        // A live jump before dead code IS collected (and stops the scan there).
        assert_eq!(
            escaping_loop_jumps(&lower_module("break\nset x 1").top_level, 0),
            (true, false),
            "live break is collected",
        );
    }

    /// Regression coverage for issue #996: `escaping_loop_jumps` and the
    /// mutually-recursive `switch_escaping_jumps` recurse once per nested
    /// `if`/`switch`/`Block`/`UpFrame`/`try` body, with no depth cap of
    /// their own before this fix. Transitively bounded to
    /// `MAX_LOWER_DEPTH` (256) by the lowering pass today, so this is
    /// defence-in-depth / consistency with every other full-tree walker in
    /// this crate, not a currently-reproducible crash. 1000 levels of
    /// source nesting is comfortably past this new cap; the assertion is
    /// that `escaping_loop_jumps` returns at all, not what it returns.
    /// Spawns its own big-stack thread since the lexer/CST/segmenter
    /// stages upstream of the lowering cap still walk the full
    /// un-truncated source nesting before that cap trims it — same
    /// rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_escaping_loop_jumps() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("break\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = escaping_loop_jumps(&lower_module(&src).top_level, 0);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn find_call_defs<'a>(func: &'a Function, command: &str) -> Option<&'a [String]> {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command: c, defs, ..
                } = stmt
                    && c == command
                {
                    return Some(defs);
                }
            }
        }
        None
    }

    #[test]
    fn detect_upvar_procs_registers_short_and_qualified() {
        let module = lower_module("proc ::ns::p {} { upvar 1 caller_x x }\nproc ::ns::p2 {} {}");
        let upvar_procs = detect_upvar_procs(&module);
        assert!(upvar_procs.contains_key("::ns::p"));
        assert!(upvar_procs.contains_key("p"));
        // p2 has no upvar — not registered.
        assert!(!upvar_procs.contains_key("::ns::p2"));
        assert!(!upvar_procs.contains_key("p2"));
    }

    // Issue #923 idx 18 (revisited after PR #1020 review): a wrapper proc
    // that reaches an already-known upvar proc through a *plain* call
    // (`real_worker $fvar $nvar $script`, not `uplevel`) does NOT itself
    // become an upvar-write target for its own caller — tclsh9.0/8.6-
    // verified (`can't read "myf": no such variable"` when the caller reads
    // the variable in a statement separate from the call, i.e. genuinely
    // outside any `uplevel`'d script argument). A plain call only shares
    // *values*, not stack frames: `real_worker`'s own `upvar 1` reaches the
    // wrapper's frame, not the wrapper's caller's frame — an earlier
    // version of this fix treated every such pass-through as transitive,
    // which was disproven by re-testing with the read moved outside the
    // uplevel'd script (see the reverted commit's own follow-up fix for
    // the story). The real tcllib idiom this finding was mined from
    // (`page::util::flow`) reaches its own worker via `uplevel 1 [list
    // ... ]`, not a plain call — genuinely propagating one frame further,
    // confirmed separately against tclsh9.0 — but modelling that shape
    // soundly (accounting for the wrapper's own uplevel level composed
    // with the callee's own upvar level) is out of scope here; tracked at
    // https://github.com/bitwisecook/tcl-lsp/issues/1019.

    #[test]
    fn detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper() {
        // TN — `wrapper` has no `upvar` of its own, and its call to the
        // known upvar proc `real_worker` is a plain call (not `uplevel`),
        // so it must NOT be registered as a transitive upvar proc.
        let module = lower_module(
            "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc wrapper {fvar nvar script} {\n\
             real_worker $fvar $nvar $script\n\
             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            !upvar_procs.contains_key("wrapper"),
            "a plain-call wrapper must not be treated as a transitive upvar proc",
        );
    }

    #[test]
    fn detect_upvar_procs_does_not_propagate_when_args_are_not_passed_through() {
        // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`,
        // but with literal args ("x"/"y"), not its own parameters passed
        // through unchanged — real tclsh9.0/8.6 confirms this genuinely
        // errors ("can't read \"myf\": no such variable"), so it must NOT
        // be registered.
        let module = lower_module(
            "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc unrelated_wrapper {a b c} {\n\
             real_worker x y $c\n\
             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            !upvar_procs.contains_key("unrelated_wrapper"),
            "unrelated_wrapper passes literal args, not its own params — must not propagate",
        );
    }

    /// Issue #1019 / issue #923 audit idx 24: the *other* half of the rule
    /// the two tests above pin. A level-**1** effect stops at the wrapper's
    /// own frame, but a level that lands past the callee's caller reaches
    /// this proc's caller through an ordinary call, so the wrapper really
    /// does become a caller-frame writer.
    ///
    /// Oracle, byte-identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```tcl
    /// proc setUp2 {var} { uplevel 2 [list set $var 99] }
    /// proc middle {}    { setUp2 answer }
    /// proc outer  {}    { middle ; return $answer }
    /// puts [outer]        ;# -> 99
    /// ```
    #[test]
    fn detect_upvar_procs_propagates_a_beyond_caller_effect_one_plain_hop() {
        let module = lower_module(
            "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        let middle = upvar_procs
            .get("middle")
            .expect("a plain call to a beyond-caller writer registers the wrapper");
        assert!(
            middle.caller_frame_opaque_writes,
            "setUp2's `uplevel 2` lands in middle's caller: {middle:?}"
        );
        assert!(
            middle.caller_frame_opaque_reads,
            "the same script may read there too: {middle:?}"
        );
    }

    /// TN — `uplevel #0` (global) and `uplevel 0` (the callee's own frame)
    /// reach no caller at all, so neither is a beyond-caller effect and
    /// neither makes a plain-call wrapper one.
    #[test]
    fn detect_upvar_procs_does_not_propagate_a_global_or_own_frame_level() {
        for level in ["#0", "0"] {
            let module = lower_module(&format!(
                "proc setSomewhere {{var}} {{\n                 uplevel {level} [list set $var 99]\n                 }}\n                 proc wrapper {{}} {{\n                 setSomewhere answer\n                 }}"
            ));
            let upvar_procs = detect_upvar_procs(&module);
            assert!(
                !upvar_procs.contains_key("wrapper"),
                "`uplevel {level}` never touches a caller frame: {upvar_procs:?}"
            );
        }
    }

    /// The published summaries carry no composition bookkeeping — they are
    /// folded into every procedure's `function_lattice` memo key, so only
    /// what a call site actually reads may travel in them.
    #[test]
    fn detect_upvar_procs_publishes_no_plain_call_bookkeeping() {
        let module = lower_module(
            "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
        );
        for (name, info) in detect_upvar_procs(&module) {
            assert!(
                info.plain_calls.is_empty(),
                "{name} published composition input: {info:?}"
            );
        }
    }

    #[test]
    fn prepare_cfg_context_registers_params_for_all_procs() {
        let module = lower_module("proc ::ns::p {a b} { upvar 1 $a x }\nproc q {c} {}");
        let (_upvar_procs, proc_params, _global_write_procs) = prepare_cfg_context(&module);
        assert_eq!(
            proc_params.get("::ns::p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        assert_eq!(
            proc_params.get("p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        // q has no upvar but its params should still be in proc_params.
        assert_eq!(proc_params.get("q"), Some(&vec!["c".to_string()]));
    }

    #[test]
    fn literal_upvar_proc_call_augments_defs() {
        // Caller invokes a proc whose body has `upvar 1 caller_x x`.
        // The call should land with `caller_x` in its defs.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs, got {defs:?}",
        );
    }

    #[test]
    fn param_upvar_proc_call_resolves_call_site_arg() {
        // `proc setter {name} { upvar 1 $name x }` aliased to whatever
        // the caller passes for `name`.  Call `setter my_var` should
        // augment the call with `my_var` in defs.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter my_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"my_var".to_string()),
            "expected my_var in defs, got {defs:?}",
        );
    }

    #[test]
    fn param_upvar_normalises_dollar_call_arg() {
        // `setter $caller_var` — the call passes a `$`-prefixed name;
        // the wiring normalises it to `caller_var` for the def list.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x }\n\
             setter $caller_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_var".to_string()),
            "expected caller_var (normalised from $caller_var) in defs, got {defs:?}",
        );
    }

    #[test]
    fn non_upvar_proc_call_unchanged() {
        // No upvar in the callee — the call's defs should be empty.
        let module = lower_module("proc no_upvar {} { set x 1 }\nno_upvar");
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "no_upvar")
            .expect("no_upvar call should be in top-level CFG");
        assert!(defs.is_empty(), "expected no augmented defs, got {defs:?}");
    }

    #[test]
    fn qualified_call_resolves_via_qualified_key() {
        // `proc ::ns::setter` is registered under both `::ns::setter`
        // and `setter`.  A qualified call site `::ns::setter` should
        // resolve via the qualified key.
        let module = lower_module(
            "proc ::ns::setter {} { upvar 1 caller_x x }\n\
             ::ns::setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "::ns::setter")
            .expect("::ns::setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs (qualified call), got {defs:?}",
        );
    }

    #[test]
    fn cross_proc_call_inside_proc_body_augments_defs() {
        // Outer proc calls an inner upvar-using proc.  The outer
        // proc's CFG should reflect the augmented defs.
        let module = lower_module(
            "proc inner {} { upvar 1 caller_x x }\n\
             proc outer {} { inner }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg
            .procedures
            .get("::outer")
            .expect("outer proc CFG should exist");
        let defs = find_call_defs(outer, "inner").expect("inner call should be in outer CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in inner's defs (called from outer), got {defs:?}",
        );
    }

    #[test]
    fn upvar_call_inside_if_branch_augments_defs() {
        // Calls inside structured constructs (if branches, while
        // bodies, ...) must also be augmented — the wiring runs in
        // `lower_script`, which is invoked recursively for every
        // body.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_y y }\n\
             if {1} { setter }",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call in if-branch should be in top-level CFG");
        assert!(
            defs.contains(&"caller_y".to_string()),
            "expected caller_y in defs (call inside if branch), got {defs:?}",
        );
    }

    #[test]
    fn empty_module_no_upvar_context_no_panic() {
        // Empty module → empty upvar_procs and proc_params.  Building
        // a CFG should still succeed.
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn build_cfg_function_does_not_apply_upvar_wiring() {
        // `build_cfg_function` is the no-context variant used by
        // tests and one-off CFG construction.  It MUST NOT augment
        // defs even when the script calls a known upvar proc —
        // the function only sees the script, not a module.
        let module = lower_module("proc setter {} { upvar 1 caller_x x }\nsetter");
        // Build only the top-level CFG via the no-context API.
        let func = build_cfg_function("::top", &module.top_level, true);
        let defs = find_call_defs(&func, "setter").expect("setter call should be in top-level CFG");
        assert!(
            defs.is_empty(),
            "build_cfg_function without context should leave defs empty, got {defs:?}",
        );
    }

    // Embedded-substitution
    // form: `[upvar_proc arg]` inside `AssignValue.value` or `Call.args`.

    /// Walk every block looking for a Call whose `defs` contain
    /// *def*.  Returns the command name when found.
    fn find_call_with_def<'a>(func: &'a Function, def: &str) -> Option<&'a str> {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { command, defs, .. } = stmt
                    && defs.iter().any(|d| d == def)
                {
                    return Some(command);
                }
            }
        }
        None
    }

    #[test]
    fn words_from_text_extracts_word_list() {
        let words = words_from_text("step 1 two");
        assert_eq!(
            words,
            vec!["step".to_string(), "1".to_string(), "two".to_string()],
        );
    }

    #[test]
    fn words_from_text_prefixes_dollar_for_vars() {
        let words = words_from_text("step $varname");
        assert_eq!(words, vec!["step".to_string(), "$varname".to_string()]);
    }

    #[test]
    fn words_from_text_handles_empty() {
        assert!(words_from_text("").is_empty());
    }

    #[test]
    fn embedded_subst_in_assign_value_emits_synthetic_invalidate() {
        // `set foo [setter]` where setter upvars caller_x.  The
        // resulting CFG should have a synthetic `<upvar-invalidate>`
        // Call with `caller_x` in its defs, emitted BEFORE the
        // `set foo ...` AssignValue.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; return $x }\n\
             set foo [setter]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "caller_x")
            .expect("expected a Call carrying caller_x in defs");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    #[test]
    fn embedded_subst_in_call_arg_merges_into_call_defs() {
        // `puts [setter]` — Call host with embedded substitution.
        // The defs should merge into the existing Call's defs (no
        // synthetic invalidate needed since the host is a Call).
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; return $x }\n\
             puts [setter]",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "puts").expect("puts call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x merged into puts's defs, got {defs:?}",
        );
        // No synthetic invalidate should appear (the Call branch
        // merged in place).
        let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
        assert_eq!(
            synthetic,
            Some("puts"),
            "embedded extras should merge into the Call host, not a synthetic",
        );
    }

    #[test]
    fn embedded_subst_param_form_resolves_call_site_arg() {
        // setter takes a parameter `name`; its upvar source is `$name`.
        // `set foo [setter myvar]` — the embedded call passes "myvar",
        // which becomes the caller-side def.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             set foo [setter myvar]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "myvar")
            .expect("expected synthetic invalidate carrying myvar");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    #[test]
    fn embedded_subst_unknown_command_ignored() {
        // `[not_upvar]` — unknown command, should produce no
        // synthetic invalidate.
        let module = lower_module("proc setter {} { set x 1 }\nset foo [setter]");
        let cfg = build_cfg(&module, false);
        // setter has no upvar, so neither direct nor embedded form
        // contributes — the only Call in the CFG should be
        // for the literal `setter` lookup (none here) or nothing.
        for block in cfg.top_level.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { command, .. } = stmt {
                    assert_ne!(
                        command, "<upvar-invalidate>",
                        "no synthetic invalidate should appear for non-upvar embedded calls",
                    );
                }
            }
        }
    }

    #[test]
    fn embedded_subst_no_bracket_in_text_short_circuits() {
        // `set foo "plain string"` — value has no `[`, so the
        // embedded-substitution scan should short-circuit on the
        // `text.contains('[')` guard and produce no extras.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x }\n\
             set foo plain",
        );
        let cfg = build_cfg(&module, false);
        let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
        assert!(
            synthetic.is_none(),
            "no synthetic invalidate expected when text has no `[`, got {synthetic:?}",
        );
    }

    #[test]
    fn embedded_subst_synthetic_appears_before_host_assign() {
        // The synthetic invalidate must land BEFORE the host
        // AssignValue in program order, so SSA / dataflow correctly
        // see the invalidation before any later use of the variable.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x }\n\
             set foo [setter]",
        );
        let cfg = build_cfg(&module, false);
        let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
        // Find the synthetic invalidate's index and the AssignValue's
        // index; assert ordering.
        let mut synthetic_idx = None;
        let mut assign_idx = None;
        for (i, stmt) in entry.statements.iter().enumerate() {
            match stmt {
                Statement::Call { command, .. } if command == "<upvar-invalidate>" => {
                    synthetic_idx = Some(i);
                }
                Statement::AssignValue { name, .. } if name == "foo" => {
                    assign_idx = Some(i);
                }
                _ => {}
            }
        }
        let s = synthetic_idx.expect("synthetic <upvar-invalidate> should be in entry block");
        let a = assign_idx.expect("set foo AssignValue should be in entry block");
        assert!(
            s < a,
            "synthetic invalidate at {s} should precede assign at {a}",
        );
    }

    // Issue #923 idx 122: a known upvar proc's call-by-name write reached
    // only through a nested `[...]` (a wrapping command around it, or a
    // loop/branch condition) must still be recovered — real tcllib
    // `cmdline::getoptions` repro: `while {[set err [getopt argv $opts
    // opt arg]]} { ... }`.

    #[test]
    fn embedded_subst_recovers_a_wrapped_upvar_proc_call() {
        // TP — `set x [set err [setter]]`: `setter` is nested one bracket
        // deeper than the embedded-substitution scan's own top-level word
        // check reaches (`set` itself is never a known upvar proc), so
        // recovering it requires recursing into the inner text rather than
        // pattern-matching the (bracket-stripped) word list.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             set out [set err [setter]]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "caller_x")
            .expect("expected a synthetic invalidate recovering caller_x through the wrapper");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    #[test]
    fn condition_out_vars_recovers_catch_scan_gets_regexp_and_upvar_procs() {
        // TP — `condition_out_vars` unions the hardcoded builtin scan with
        // the upvar-proc / global-write-proc resolution; a known upvar
        // proc's write reached through a condition must appear in the
        // union exactly like a `catch` result var already does.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             if {[setter]} { puts $caller_x }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in the <cond> Call's defs, got {defs:?}",
        );
    }

    #[test]
    fn frozen_while_condition_carries_upvar_proc_defs() {
        // TP — `while {[getopt ...]}`'s condition is purely a command
        // substitution, so `lower_while_or_frozen` freezes the whole loop
        // as an opaque `Statement::Barrier` (no `defs` field at all) rather
        // than calling `lower_while`. Without a separate synthetic `<cond>`
        // Call carrying the condition's defs, `getopt`'s upvar write to
        // `opt` would be invisible to the def-use graph even though the
        // barrier's own `uses_of` textually scans the (un-lowered) body
        // for `$opt` and attributes the read to the same statement.
        let module = lower_module(
            "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             while {[getopt opt]} { puts $opt }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"opt".to_string()),
            "expected opt in the frozen while's <cond> Call defs, got {defs:?}",
        );
        // The barrier itself must still be there too — freezing, not
        // ordinary `lower_while`, is exactly the shape under test.
        let has_barrier = cfg.top_level.blocks.values().any(|b| {
            b.statements
                .iter()
                .any(|s| matches!(s, Statement::Barrier { command, .. } if command == "while"))
        });
        assert!(
            has_barrier,
            "expected the loop to still freeze as a Barrier"
        );
    }

    #[test]
    fn frozen_for_condition_carries_upvar_proc_defs() {
        // TP — the `for` loop's identical frozen-barrier path (issue #923
        // idx 122 applies equally to `for {...} [cond] {...} {...}`).
        let module = lower_module(
            "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             for {} {[getopt opt]} {} { puts $opt }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"opt".to_string()),
            "expected opt in the frozen for's <cond> Call defs, got {defs:?}",
        );
    }

    #[test]
    fn frozen_while_condition_with_no_upvar_proc_omits_synthetic_cond() {
        // TN — a frozen loop whose condition calls an *ordinary* (non-
        // upvar, non-`catch`/`scan`/`gets`/`regexp`) command must not gain
        // a synthetic `<cond>` Call at all — nothing for it to carry.
        let module = lower_module("proc pureCheck {} { return 1 }\nwhile {[pureCheck]} { }");
        let cfg = build_cfg(&module, false);
        assert!(
            find_call_defs(&cfg.top_level, "<cond>").is_none(),
            "no synthetic <cond> Call expected when the condition defines nothing",
        );
    }

    #[test]
    fn try_handler_return_options_terminates() {
        // A `try` handler ending in `return -code error …` (lowered to a
        // "return with options" barrier) returns from the proc, so it must
        // *terminate* its block rather than fall through to the post-`try`
        // join. Regression for the spurious `try_handler → try_end` edge +
        // phi (e.g. `auto_mkindex`). Analysis builds only (`build_cfg` sets
        // `faithful_exceptions`); codegen leaves the barrier as a plain stmt.
        let module =
            lower_module("proc f {} { try { set a 1 } on error {} { return -code error boom } }");
        let cfg = build_cfg(&module, false);
        let func = cfg.procedures.get("::f").expect("::f cfg");
        let handler = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("try_handler"))
            .expect("try_handler block");
        assert!(
            matches!(handler.terminator, Some(Terminator::Return { .. })),
            "return-options handler must terminate (no fall-through to try_end), got {:?}",
            handler.terminator,
        );
    }
}
