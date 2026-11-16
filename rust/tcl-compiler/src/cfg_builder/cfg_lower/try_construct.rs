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

//! Flattening `try`: the body region, handler dispatch, completion codes and the `finally` exits.

use super::super::CfgBuilder;
use crate::cfg::Terminator;
use crate::ir::Statement;
use tcl_registry::value_transfer::completion::{HandlerChain, HandlerLink};

impl CfgBuilder<'_> {
    // try

    /// Record the analysis-only exception edges into a `try` handler block:
    ///
    /// * `on ok` runs only after the body completes normally → source = body
    ///   tail (the body's exit versions).
    /// * a body with *no* normal fall-through (it unconditionally throws /
    ///   returns) reaches the handler only from its explicit throw points (at
    ///   their throw-time versions), falling back to the terminal block when the
    ///   body terminated without an explicit `error`/`throw`.
    /// * a body that *can* fall through may complete abnormally at any point, so
    ///   a body-set var is only *maybe* defined → merge the pre-`try` state
    ///   (version-0) with the body-exit state.
    pub(super) fn push_try_handler_exception_edges(
        &mut self,
        (chain, group): (&HandlerChain, &[usize]),
        (handler_block, block_name, body_block): (&str, &str, &str),
        body_tail: Option<&str>,
        (body_throw_blocks, split_points): (&[String], &[super::super::SplitPoint]),
        (body_terminal, first): (Option<&str>, Option<&Statement>),
    ) {
        if !self.faithful_exceptions || group.is_empty() {
            return;
        }
        // `group` is every handler whose match runs this block's body: a `-`
        // handler's own block is empty, so the body it shares is reached only
        // through the edges of the handler that owns it. It runs after the
        // body completes normally alone when every member selects `ok` — the
        // registry's completion-code parse, so `0` is `ok` too. `try` declares
        // no default clause: `ok` is a value of the pattern word, not "no
        // handler matched".
        let is_on_ok = group
            .iter()
            .all(|&member| chain.takes(member, tcl_core_types::Code::Ok));
        if is_on_ok {
            if let Some(tail) = body_tail {
                self.exception_edges
                    .push((tail.to_owned(), handler_block.to_owned()));
            }
        } else if body_tail.is_none() {
            let mut throw_sources: Vec<String> = Vec::new();
            for tb in body_throw_blocks {
                if !throw_sources.contains(tb) {
                    throw_sources.push(tb.clone());
                }
            }
            if throw_sources.is_empty()
                && let Some(terminal) = body_terminal
            {
                throw_sources.push(terminal.to_owned());
            }
            // A source whose completion the registry knows exactly reaches
            // only a handler that selects that code. Wiring
            // `try {break} on error {} {}`, `on continue`, or
            // `try {return early} on error {} {}` to the handler made the
            // `try` look as if it could complete normally (found in review).
            // A selector or completion the registry cannot decode keeps the
            // edge, and so does a match by any handler of a `-` group:
            // `try {error boom} on error {} - on ok {} {set x 1}` runs
            // `set x 1`, which tclsh 8.6.18 and 9.0.4 confirm (found in
            // review).
            throw_sources.retain(|src| {
                group
                    .iter()
                    .any(|&member| !self.handler_misses_completion(chain, member, src))
            });
            for src in throw_sources {
                self.exception_edges.push((src, handler_block.to_owned()));
            }
            // A body that never rests still starts somewhere: a first command
            // other than a literal assignment may fail before it stores, and
            // the handler then sees the state before the body, over the region
            // entry the solver opens there, as for a body that rests below —
            // unless the body's first block completes with a code the registry
            // knows exactly and no member takes, as the throw sources above
            // are filtered. A literal assignment needs no entry: it raises
            // only where its own place holds an array, whose scalar value
            // nothing reads, and leaves every other place as the point after
            // it does. Without the entry the handler took what the first block
            // left: `try {lassign {x y} a b; error boom} on error {} {}` gave it
            // `b` as `lassign` wrote it where `a` may be an array, and `set b
            // old` before it went as dead.
            let entry = (block_name.to_owned(), handler_block.to_owned());
            let first_may_fail = first.is_some_and(|first| {
                !self.leaves_from_the_state_before(first)
                    && super::super::NextFailure::of(first) == super::super::NextFailure::Any
            });
            let reached = group
                .iter()
                .any(|&member| !self.handler_misses_completion(chain, member, body_block));
            if first_may_fail && reached && !self.exception_edges.contains(&entry) {
                self.exception_edges.push(entry);
                self.region_entries.push((
                    block_name.to_owned(),
                    handler_block.to_owned(),
                    body_block.to_owned(),
                ));
            }
            self.push_split_failure_edges(handler_block, split_points, |code| {
                group.iter().any(|&member| !chain.misses(member, code))
            });
        } else {
            self.exception_edges
                .push((block_name.to_owned(), handler_block.to_owned()));
            self.region_entries.push((
                block_name.to_owned(),
                handler_block.to_owned(),
                body_block.to_owned(),
            ));
            if let Some(tail) = body_tail
                && tail != block_name
            {
                self.exception_edges
                    .push((tail.to_owned(), handler_block.to_owned()));
            }
            // A resting tail does not mean the explicit throws went unseen:
            // one inside a nested `if`, or a `finally` that only ever resumes
            // unwinding, raises with its own block's defs live. Sourcing only
            // the pre-`try` block and the tail read `x` as unset after
            // `try { if {$c} {set x 1; error b} else {set x 2; error c} }
            // on error {} {}` and called its stores dead.
            let mut seen: Vec<&String> = Vec::new();
            for tb in body_throw_blocks {
                if tb != block_name && body_tail != Some(tb.as_str()) && !seen.contains(&tb) {
                    seen.push(tb);
                    self.exception_edges
                        .push((tb.clone(), handler_block.to_owned()));
                }
            }
            self.push_split_failure_edges(handler_block, split_points, |code| {
                group.iter().any(|&member| !chain.misses(member, code))
            });
        }
    }

    /// Record an exception edge into `target` from each point a throw may
    /// leave the body from for it, where the edge is not there already. The
    /// state at the point is the one a statement before it completes with when
    /// it leaves abnormally after its own stores, with any code, and the one
    /// the next statement fails with when it fails before it stores — an error
    /// alone from a literal assignment, for which `takes` says whether the
    /// target may take it. A codegen build records none.
    fn push_split_failure_edges(
        &mut self,
        target: &str,
        points: &[super::super::SplitPoint],
        takes: impl Fn(tcl_core_types::Code) -> bool,
    ) {
        if !self.faithful_exceptions {
            return;
        }
        for point in points {
            if !point.after_stores && !point.next_fails_for(&takes) {
                continue;
            }
            let edge = (point.block.clone(), target.to_owned());
            if !self.exception_edges.contains(&edge) {
                self.exception_edges.push(edge);
            }
        }
    }

    /// Record the ways a failure of the body reaches the `finally` clause when
    /// no handler takes it: from each point a throw may leave the body from,
    /// and from the block before the body, an edge that opens where the body's
    /// first command can fail before it stores (a region entry). A command
    /// may fail with any code and no handler takes every code, so a handler
    /// stands in for a point only where the failure is an error a handler
    /// certainly takes ([`HandlerChain::first_taking`]). A first statement
    /// that completes with its code from the state before it — `error boom`,
    /// `exit 7` — leaves by its own block, which the clause is wired from
    /// already, and one that runs a clause of its own first leaves through it.
    /// Without these edges the clause ran over the state the body ends in
    /// alone: `try {set x 2; foo; set x 3} finally {puts $x}` printed `3`
    /// once optimised where tclsh prints `2` when `foo` raises.
    fn push_finally_failure_edges(
        &mut self,
        (end_block, chain): (&str, &HandlerChain),
        (block_name, body_block, first): (&str, &str, Option<&Statement>),
        points: &[super::super::SplitPoint],
    ) {
        if !self.faithful_exceptions {
            return;
        }
        let escapes = |code| chain.first_taking(code).is_none();
        let entry = (block_name.to_owned(), end_block.to_owned());
        let first_fails = first.is_some_and(|first| {
            !self.leaves_from_the_state_before(first)
                && match super::super::NextFailure::of(first) {
                    super::super::NextFailure::Intercepted => false,
                    super::super::NextFailure::Error => escapes(tcl_core_types::Code::Error),
                    super::super::NextFailure::Any => true,
                }
        });
        if first_fails && !self.exception_edges.contains(&entry) {
            self.exception_edges.push(entry);
            self.region_entries.push((
                block_name.to_owned(),
                end_block.to_owned(),
                body_block.to_owned(),
            ));
        }
        self.push_split_failure_edges(end_block, points, escapes);
    }

    /// Hand the ways the body of a `try` with no `finally` can leave to the
    /// region around it: a completion none of its handlers takes leaves the
    /// `try` from where the body left, so each of the body's points is a way
    /// out of the enclosing body too. A `try` with a `finally` runs it first,
    /// and the clause's own blocks are the enclosing region's.
    fn escape_to_the_region_around(
        &mut self,
        points: &[super::super::SplitPoint],
        throw_blocks: &[String],
        ends: [Option<&str>; 2],
    ) {
        if !self
            .split_region
            .is_some_and(super::super::SplitRegion::nested)
        {
            return;
        }
        let ways_out = throw_blocks
            .iter()
            .map(String::as_str)
            .chain(ends.into_iter().flatten())
            .map(|block| super::super::SplitPoint::any(block.to_owned()));
        for point in points.iter().cloned().chain(ways_out) {
            if !self
                .split_points
                .iter()
                .any(|known| known.block == point.block)
            {
                self.split_points.push(point);
            }
        }
    }

    /// The exact completion code with which *every* path through `block`
    /// leaves, when it is known: the [`terminal code`](Self::terminal_code)
    /// of a block with nothing else in it. An earlier statement may complete
    /// first — `set y $x; return ok` raises when `x` is unset, which an
    /// `on error` handler catches — so a block with one proves no single code
    /// (found in review).
    ///
    /// Nor may anything before the block: `try {if {$c} {}; return ok}` puts
    /// the `return` alone in `if_end`, but `$c` may raise first, and a failure
    /// in an earlier block has no edge of its own — the terminal block carries
    /// it (found in review). So only `entry`, the construct's own first block,
    /// qualifies — with the steps an analysis build splits the construct's
    /// first statements into after it, whose statements are read as one run
    /// ([`Self::entry_run_statements`]).
    fn block_completion_code(&self, block: &str, entry: &str) -> Option<tcl_core_types::Code> {
        let statements = self.entry_run_statements(block, entry)?;
        if self.plain_return_blocks.contains(block) {
            return statements
                .is_empty()
                .then(|| self.terminal_code(block))
                .flatten();
        }
        let code = self.terminal_code(block)?;
        let [before @ .., _] = statements.as_slice() else {
            return None;
        };
        // A literal assignment completes normally or raises `TCL_ERROR` (the
        // name is an array, a write trace fails) — never another code — so it
        // cannot change an error's code: `set z 0; error boom` is caught by
        // `on error` whichever raises (found in review). A command
        // substitution in an unbraced name could complete with any code; a
        // braced name (`set {[} 0`) substitutes nothing.
        let only_errors_before = before.iter().all(|stmt| {
            matches!(stmt, Statement::AssignConst { name, name_braced, .. }
                if *name_braced || !name.contains('['))
        });
        (before.is_empty() || (code == tcl_core_types::Code::Error && only_errors_before))
            .then_some(code)
    }

    /// The statements of `block` and of the blocks before it on the straight
    /// run of split steps that begins at `entry`, in order: the construct's
    /// first statements, which an analysis build gives a block each. `None`
    /// when `block` is not on that run — a compound statement ends it, so in
    /// `if {$c} {}; return ok` the `return` is past a block `$c` may raise in.
    fn entry_run_statements(&self, block: &str, entry: &str) -> Option<Vec<&Statement>> {
        let mut run = vec![block];
        let mut at = block;
        while at != entry {
            at = self.split_parent.get(at)?;
            run.push(at);
        }
        let mut statements = Vec::new();
        for name in run.iter().rev() {
            statements.extend(self.blocks.get(*name)?.statements.iter());
        }
        Some(statements)
    }

    /// The completion code of whatever ended `block`, when it is known: a
    /// plain `return`, or a last statement the registry decodes (see
    /// [`super::super::exact_statement_completion`]) that is what ended the block — a
    /// `break` / `continue` behind its `Goto`, or a non-`ok` code behind a
    /// `Return`. A block ended some other way (an opaque `switch`, or a
    /// `finally` clause resuming what it interrupted) has no single code.
    /// Earlier statements in the block are not considered.
    fn terminal_code(&self, block: &str) -> Option<tcl_core_types::Code> {
        use tcl_core_types::Code;
        if self.plain_return_blocks.contains(block) {
            return Some(Code::Return);
        }
        let mutable = self.blocks.get(block)?;
        if let Some(Terminator::Complete { route, .. }) = &mutable.terminator {
            return route.immediate_code();
        }
        let stmt = mutable.statements.last()?;
        let resolve = self.embedded_head_resolver();
        let tcl_registry::registry::ExactInvocationCompletion::Tcl(code) =
            self.command_classes.exact_completion(stmt, &resolve)?
        else {
            return None;
        };
        match (&mutable.terminator, code) {
            (Some(Terminator::Goto { .. }), Code::Break | Code::Continue) => Some(code),
            (Some(Terminator::Return { .. }), code)
                if !matches!(code, Code::Ok | Code::Break | Code::Continue) =>
            {
                Some(code)
            }
            _ => None,
        }
    }

    /// The completion code named by the handler under its retained source
    /// numeral grammar. Catalogue availability does not choose this grammar;
    /// unavailable input permits only selectors on which all grammars agree.
    #[cfg(test)]
    pub(super) fn handler_code(
        &self,
        handler: &crate::ir::TryHandler,
    ) -> Option<tcl_core_types::Code> {
        crate::executable_ir::try_handler_code_in(
            handler,
            self.command_classes.source_numbers(self.config),
        )
    }

    /// The handlers of a `try` as the registry reads them
    /// ([`HandlerChain`]): each selector decoded with the target's own numeral
    /// grammar, so `on 010` is code 8 in Tcl 8.x and 10 in 9.0 (decoding it as
    /// 9.0 dropped a live 8.x handler — found in review). Which handler a
    /// completion reaches, which can never run and which scripts a `-` handler
    /// shares are the chain's answers; nothing here reads the list again.
    pub(super) fn handler_chain(&self, handlers: &[crate::ir::TryHandler]) -> HandlerChain {
        HandlerChain::new(
            handlers.iter().map(|handler| HandlerLink {
                matches: handler.kind,
                selector: &handler.match_arg,
                falls_through: handler.fallthrough,
            }),
            self.command_classes.source_numbers(self.config),
        )
    }

    /// Whether `block` ends in a statement whose completion code handler
    /// `member` is known not to select: the block's code known exactly
    /// ([`Self::block_completion_code`]), and the handler's selector naming
    /// another ([`HandlerChain::misses`]). Either one unknown answers `false`,
    /// keeping the edge.
    fn handler_misses_completion(&self, chain: &HandlerChain, member: usize, block: &str) -> bool {
        self.try_entry
            .as_deref()
            .and_then(|entry| self.block_completion_code(block, entry))
            .is_some_and(|code| chain.misses(member, code))
    }

    /// Record the analysis-only edges that keep a `finally` clause reachable
    /// from every way the `try` body or one of its handlers can leave.
    ///
    /// `lower_try` gave `end_block` a predecessor from a body that falls through
    /// normally, or from a handler's throw edge. Nothing connected a body exit
    /// that is neither — a `return`, `error`, `throw`, or a `break`/`continue`
    /// out of the body — so a `finally` reached only that way read as dead, and
    /// O107 emptied it:
    ///
    /// ```tcl
    /// set g 0
    /// proc p {} { global g; try {error boom} finally {set g 1} }
    /// catch {p}; puts $g
    /// ```
    ///
    /// printed `0` where tclsh 8.6.18 and 9.0.4 print `1` (#2142). The exits
    /// are read off the body's own blocks rather than its resting tail: a body
    /// whose every branch leaves — `if {$c} {return ok} else {error boom}` —
    /// still ends in a resting `if_end` block, so "has a tail" is not "can fall
    /// through", and gating on it left that `finally` dead too.
    ///
    /// Not added without a `finally`: there the tail really is unreachable on
    /// these paths, because the exception resumes unwinding past it.
    ///
    /// A `break` or `continue` is different: the clause runs and then the jump
    /// goes on to its loop target. The jump is retargeted at `end_block` and
    /// the saved targets are returned for [`Self::lower_try`] to resume from
    /// the clause's last block. Adding an edge alongside the jump left a
    /// path into the loop that skipped the clause, and
    /// `while {$first || $x} { try {set first 0; continue} finally {set x 0} }`
    /// reported `x` read before it is set (found in review). The same holds
    /// for a jump a nested `try` has already resumed after its own clause.
    ///
    /// What it does cost, when the body or a handler can *also* complete
    /// normally, is that the clause's normal exit into `try_after_finally` is
    /// shared by the exit paths, where Tcl in fact keeps unwinding or jumps.
    /// Modelling that exactly needs the clause body lowered once per way in;
    /// the merge only adds paths, and over-approximating the *other* way — a
    /// `finally` clause that is never entered — is what corrupted the program
    /// above. When nothing can complete normally there is no merge:
    /// [`Self::lower_try_finally`] ends the clause as an exit.
    fn push_finally_exit_edges(
        &mut self,
        end_block: &str,
        post_body: &str,
        body_block: &str,
        first_body_id: usize,
        chain: &HandlerChain,
        handler_blocks: &[String],
    ) -> (Vec<String>, bool) {
        if !self.faithful_exceptions {
            return (Vec::new(), false);
        }
        let end_id = self.bid(end_block);
        let body_block_id = self.bid(body_block);
        // The body's blocks *and* the handlers': both are lowered after
        // `first_body_id`, and a handler that leaves — `on error {} {return
        // handled}` — is as much an exit as the body's own `return`.
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        // Normal completion already reaches `try_end`, directly or through
        // `try_ok`; a jump there is not an exit to wire a second time.
        let completion = [end_id, self.bid(post_body)];
        let leaves = |t: crate::cfg::BlockId| !in_body(t) && !completion.contains(&t);
        // A `return` a nested `catch` or `try … finally` inside this body
        // already intercepts is not an exit of *this* body: the inner one was
        // lowered first and recorded its own edge, and control reaches this
        // `finally` only after the inner clause has run — through the inner
        // construct's normal flow. Wiring it here too would skip the inner
        // `finally`, and `try { try {return ok} finally {set x 1} }
        // finally {puts $x}` read `x` as possibly unset (found in review). A
        // jump needs no such care: rerouting it only replaces an edge that
        // skipped this clause with one through it.
        let intercepted = self.totally_intercepted(&in_body, true);
        let mut sources: Vec<String> = Vec::new();
        let mut jumps: Vec<String> = Vec::new();
        let resolve_head = self.embedded_head_resolver();
        for (name, _) in self.block_ids.iter().filter(|(_, id)| in_body(**id)) {
            let Some(block) = self.blocks.get(name.as_str()) else {
                continue;
            };
            // A nested clause that resumes unwinding once done is an exit of
            // this body too, beside its fall-through.
            if self.unwinding_tails.contains(name) && !intercepted.contains(name.as_str()) {
                sources.push(name.clone());
            }
            match &block.terminator {
                // A process exit runs no `finally` (found in review) — only
                // when nothing can raise before it: it is the only statement
                // of the construct's first block, the body's or a handler's.
                // An earlier statement, or an earlier block (`if {$c} {};
                // exit 0` evaluates `$c` first), may raise an error, and an
                // error does run the clause. So may a handler's binding of its
                // result or options variable: a write trace, or an `upvar` to
                // an array, rejects it before the body runs (found in review).
                // The shared source context owns exact process completion.
                Some(
                    crate::cfg::Terminator::Return { .. } | crate::cfg::Terminator::Complete { .. },
                ) => {
                    if !intercepted.contains(name.as_str())
                        && !self.caught_by_handler(name, body_block, chain, handler_blocks)
                        && !((name == body_block || handler_blocks.contains(name))
                            && matches!(block.statements.as_slice(), [only]
                                if self.command_classes.exits_process(only, &resolve_head)))
                    {
                        sources.push(name.clone());
                    }
                }
                Some(crate::cfg::Terminator::Goto { target, .. }) if leaves(*target) => {
                    jumps.push(name.clone());
                }
                Some(crate::cfg::Terminator::Branch {
                    true_target,
                    false_target,
                    ..
                }) if leaves(*true_target) || leaves(*false_target) => {
                    jumps.push(name.clone());
                }
                _ => {}
            }
        }
        drop(resolve_head);
        let names: rustc_hash::FxHashMap<crate::cfg::BlockId, String> = self
            .block_ids
            .iter()
            .map(|(name, id)| (*id, name.clone()))
            .collect();
        let mut targets: Vec<String> = Vec::new();
        let mut resume = |t: &mut crate::cfg::BlockId| {
            if leaves(*t) {
                targets.push(names[t].clone());
                *t = end_id;
            }
        };
        jumps.sort();
        for name in &jumps {
            match &mut self.block_mut(name).terminator {
                Some(crate::cfg::Terminator::Goto { target, .. }) => resume(target),
                Some(crate::cfg::Terminator::Branch {
                    true_target,
                    false_target,
                    ..
                }) => {
                    resume(true_target);
                    resume(false_target);
                }
                _ => {}
            }
        }
        // A jump a nested `try … finally` resumes after its own clause is
        // still a jump out of this body, and must pass this clause too.
        let (nested, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.finally_jump_edges)
            .into_iter()
            .partition(|(from, to)| {
                self.block_ids.get(from).is_some_and(|id| in_body(*id))
                    && self.block_ids.get(to).is_some_and(|id| leaves(*id))
            });
        self.finally_jump_edges = kept;
        let unwinds = !sources.is_empty();
        for edge in nested {
            self.exception_edges.retain(|e| *e != edge);
            sources.push(edge.0);
            targets.push(edge.1);
        }
        sources.sort();
        sources.dedup();
        for src in sources {
            self.exception_edges.push((src, end_block.to_owned()));
        }
        targets.sort();
        targets.dedup();
        (targets, unwinds)
    }

    /// Lower the body of the `try` `stmt` from `body_block` and send a body that
    /// falls through on to `post_body`, returning what its handlers and its
    /// `finally` clause are wired from. With no `finally`, the body's points
    /// are ways out of the region around it too.
    fn lower_try_body(&mut self, stmt: &Statement, body_block: &str, post_body: &str) -> TryBody {
        let Statement::Try {
            body,
            body_span,
            finally_body,
            ..
        } = stmt
        else {
            unreachable!("lower_try_body called with non-Try");
        };
        // Install a fresh throw-block list around the body so on-error edges
        // are sourced from each explicit `error`/`throw` point (where the
        // body's prior defs are live), not the pre-`try` block. Restore the
        // outer list afterwards so a nested `try`'s throws aren't attributed
        // to this handler.
        let outer_throw_blocks = self.throw_blocks.take();
        self.throw_blocks = Some(Vec::new());
        // Any command of the body, at any depth, may fail, and what the body
        // has stored when it does is what a handler or the `finally` clause
        // runs over: in an analysis build each statement of the body and of
        // the scripts it holds ends a block an exception edge leaves from.
        let outer_region = std::mem::replace(
            &mut self.split_region,
            self.faithful_exceptions
                .then_some(super::super::SplitRegion::Try),
        );
        let outer_split_points = std::mem::take(&mut self.split_points);
        let raw_body_tail = self.lower_script(body, body_block);
        // Capture the body's terminating block *before* the handler bodies are
        // lowered below (each overwrites `last_terminal_block`).  Used to source
        // an on-error edge from a body that ended without an explicit
        // `error`/`throw` (a bare `return`).
        let body_terminal = self.last_terminal_block.take();
        let split_points = std::mem::replace(&mut self.split_points, outer_split_points);
        self.split_region = outer_region;
        let body_throw_blocks = self.throw_blocks.take().unwrap_or_default();
        self.throw_blocks = outer_throw_blocks;
        // `lower_script` now always returns the resting block, so distinguish
        // *normal fall-through* from a
        // terminated body via `body_terminal` (set iff the body did not fall
        // through — the former `body_tail.is_none()` signal). A terminated body
        // must not edge to `post_body`, and the handler's on-error edge must be
        // sourced from the throw block(s), not the pre-`try` block.
        let body_tail = if body_terminal.is_none() {
            raw_body_tail
        } else {
            None
        };
        if let Some(tail) = &body_tail {
            self.ensure_goto(tail, post_body, Some(*body_span));
        }
        if finally_body.is_none() {
            self.escape_to_the_region_around(
                &split_points,
                &body_throw_blocks,
                [body_terminal.as_deref(), body_tail.as_deref()],
            );
        }
        TryBody {
            body_tail,
            body_terminal,
            body_throw_blocks,
            split_points,
        }
    }

    /// Flatten `Statement::Try` into body → handlers → finally → end CFG.
    pub(in super::super) fn lower_try(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Try {
            span,
            body,
            handlers,
            finally_body,
            finally_span,
            ..
        } = stmt
        else {
            unreachable!("lower_try called with non-Try");
        };

        let body_block = self.new_block("try_body");
        let end_block = self.new_block("try_end");
        self.copy_command_boundary(block_name, &body_block);

        self.ensure_goto(block_name, &body_block, Some(*span));

        // Where does control go after body succeeds?
        let post_body = if handlers.is_empty() {
            end_block.clone()
        } else {
            self.new_block("try_ok")
        };

        let first_body_id = self.block_ids.len();
        let TryBody {
            body_tail,
            body_terminal,
            body_throw_blocks,
            split_points,
        } = self.lower_try_body(stmt, &body_block, &post_body);

        // A `-` (fallthrough) handler shares the next non-`-` handler's body.
        // Tcl binds the *matching* handler's variables when running that shared
        // body in the byte-compiled form, but the *target* handler's variables
        // in the interpreted form (the two diverge).
        // Statically we can't know which handler matched, so the shared body is
        // analysed with the whole group's variables treated as defined: the
        // precise over-approximation that avoids a read-before-set false
        // positive under either binding rule.
        let mut pending_fallthrough_defs: Vec<String> = Vec::new();
        let first_handler_id = self.block_ids.len();
        let mut handler_blocks: Vec<String> = Vec::new();
        let outer_entry = self.try_entry.replace(body_block.clone());
        let chain = self.handler_chain(handlers);

        // Each handler reachable from body failure.
        for (index, handler) in handlers.iter().enumerate() {
            let handler_block = self.new_block("try_handler");
            handler_blocks.push(handler_block.clone());
            self.ensure_goto(block_name, &handler_block, Some(*span));

            // Record throw edges into the handler (analysis builds only):
            // `block_name` already gotos `try_body`.
            let live_group = chain.live_group(index);
            self.push_try_handler_exception_edges(
                (&chain, &live_group),
                (&handler_block, block_name, &body_block),
                body_tail.as_deref(),
                (&body_throw_blocks, &split_points),
                (body_terminal.as_deref(), body.statements.first()),
            );

            let var_defs = handler_var_defs(handler, &mut pending_fallthrough_defs);
            self.push_handler_var_defs(&handler_block, var_defs, *span);

            if let Some(tail) = self.lower_script(&handler.body, &handler_block) {
                self.ensure_goto(&tail, &end_block, Some(handler.body_span));
            }
        }

        self.try_entry = outer_entry;
        // A split body's last step, where it leaves in a straight line, is the
        // block a handler may catch whole.
        let run_end = body_terminal.clone().unwrap_or_else(|| body_block.clone());
        if self.caught_by_handler(&run_end, &body_block, &chain, &handler_blocks) {
            self.handler_caught.insert(run_end);
        }
        // Success path reaches end.
        if !handlers.is_empty() {
            self.ensure_goto(&post_body, &end_block, Some(*span));
        }
        self.route_caught_loop_jumps(
            &chain,
            &handler_blocks,
            &body_block,
            first_body_id,
            first_handler_id,
        );

        // Finally block.
        let Some(fb) = finally_body else {
            return end_block;
        };
        self.total_interceptors.insert(end_block.clone());
        // Read before the exit edges below add paths into `end_block`.
        let completes_normally = self.try_completes_normally(
            block_name,
            &end_block,
            &post_body,
            &body_block,
            first_body_id,
        );
        let (jump_targets, unwinds) = self.push_finally_exit_edges(
            &end_block,
            &post_body,
            &body_block,
            first_body_id,
            &chain,
            &handler_blocks,
        );
        self.push_finally_failure_edges(
            (&end_block, &chain),
            (block_name, &body_block, body.statements.first()),
            &split_points,
        );
        self.finish_try_finally(
            fb,
            finally_span.or(Some(*span)),
            &end_block,
            completes_normally.then_some(unwinds),
            jump_targets,
        )
    }

    /// Lower a `try`'s `finally` clause and resume the loop jumps routed
    /// through it, returning the block the whole statement rests in.
    ///
    /// A saved jump resumes from the clause's own last block, not from
    /// `after_finally`: the statements after the `try` are appended there,
    /// and a `break` does not run them (found in review). A clause that
    /// cannot complete normally resumes nothing.
    ///
    /// `falls_through` is `None` when nothing completes the `try` normally,
    /// else whether an unwinding exit (a `return`, an error) also enters the
    /// clause. Then the clause's fall-through `Goto` stands only for normal
    /// completion: the `return` resumes past the statements after the `try`,
    /// so the last block is recorded as an unwinding tail and a throw point
    /// for the constructs around it. Without that, `try { try {if {$c}
    /// {return}} finally {}; set x 1 } finally {puts $x}` ran `set x 1` on
    /// the `return` path too, and O102 forwarded `1` into a read tclsh 8.6.18
    /// and 9.0.4 fail on (found in review).
    fn finish_try_finally(
        &mut self,
        body: &crate::ir::Script,
        fin_span: Option<tcl_lexer::Span>,
        end_block: &str,
        falls_through: Option<bool>,
        jump_targets: Vec<String>,
    ) -> String {
        let (after_finally, finally_tail) = self.lower_try_finally(
            body,
            fin_span,
            end_block,
            falls_through.is_some() || !self.faithful_exceptions,
        );
        if let Some(tail) = finally_tail {
            if falls_through == Some(true) {
                self.unwinding_tails.insert(tail.clone());
                if let Some(blocks) = self.throw_blocks.as_mut() {
                    blocks.push(tail.clone());
                }
            }
            for target in jump_targets {
                let edge = (tail.clone(), target);
                self.exception_edges.push(edge.clone());
                self.finally_jump_edges.push(edge);
            }
        }
        after_finally
    }

    /// Send a `break` / `continue` out of the body into the handler that
    /// catches it, instead of to its loop target.
    ///
    /// Tcl runs the first handler whose selector matches the completion, so a
    /// jump an `on break` handler selects never reaches the loop: in
    /// `while 1 { try {break} on break {} {set x 1} finally {}; break }` the
    /// handler binds `x` before the loop is left. Keeping the jump's own edge
    /// let the read after the loop see `x` unset, and the `finally` routing
    /// resumed it too (found in review). Only a jump out of the body (not one
    /// inside a nested loop, and not one in a handler) is caught; and a
    /// handler met first whose selector the registry cannot decode might catch
    /// it instead, so then the jump keeps its edge. A `-` handler hands the
    /// jump to the body it shares.
    fn route_caught_loop_jumps(
        &mut self,
        chain: &HandlerChain,
        handler_blocks: &[String],
        body_block: &str,
        first_body_id: usize,
        first_handler_id: usize,
    ) {
        if !self.faithful_exceptions || chain.is_empty() {
            return;
        }
        let body_block_id = self.bid(body_block);
        let in_try = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id
                || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id && i < first_handler_id)
        };
        // A jump a nested `catch` (or a nested `try … finally`) swallows never
        // reaches this `try`'s handlers: `try {catch {break}; return}
        // on break {} {…}` runs no handler (found in review).
        let intercepted = self.totally_intercepted(&in_body, false);
        let mut retargets: Vec<(String, String)> = Vec::new();
        for (name, id) in &self.block_ids {
            if !in_body(*id) || intercepted.contains(name.as_str()) {
                continue;
            }
            let Some(Terminator::Goto { target, .. }) = self
                .blocks
                .get(name.as_str())
                .and_then(|b| b.terminator.as_ref())
            else {
                continue;
            };
            if in_try(*target) {
                continue;
            }
            // The `Goto` stands for the jump alone; an earlier statement's
            // failure has its own handler edges.
            let Some(jump) = self.terminal_code(name) else {
                continue;
            };
            if !matches!(
                jump,
                tcl_core_types::Code::Break | tcl_core_types::Code::Continue
            ) {
                continue;
            }
            if let Some(first) = chain.first_taking(jump) {
                retargets.push((name.clone(), handler_blocks[chain.owner(first)].clone()));
            }
        }
        retargets.sort();
        for (name, handler_block) in retargets {
            let target = self.bid(&handler_block);
            if let Some(Terminator::Goto { target: t, .. }) = &mut self.block_mut(&name).terminator
            {
                *t = target;
            }
        }
    }

    /// Whether one of this `try`'s handlers catches every completion `block`
    /// can leave with: the block's exact completion code is known, and it has
    /// an edge into a handler whose decoded selector is that code. Control
    /// then reaches the `finally` through the handler, and a direct edge would
    /// add a path where the error escaped uncaught — which made
    /// `try {error boom} on error {} {set f 2} finally {set f 1}` read the
    /// handler's dead store as live. A block whose completion is not exact
    /// (`return $x` may raise while substituting) keeps its own exit, and so
    /// does one whose only matching handler is a `trap`: its `-errorcode`
    /// prefix may not match, and `try {error boom} trap {NOT MATCHING} {}
    /// {exit 0} finally {…}` runs the clause (found in review).
    fn caught_by_handler(
        &self,
        block: &str,
        body_block: &str,
        chain: &HandlerChain,
        handler_blocks: &[String],
    ) -> bool {
        let Some(code) = self.block_completion_code(block, body_block) else {
            return false;
        };
        // A `-` handler's match runs its owner's block.
        (0..chain.len()).any(|index| {
            chain.takes(index, code)
                && self
                    .exception_edges
                    .iter()
                    .any(|(from, to)| from == block && *to == handler_blocks[chain.owner(index)])
        })
    }

    /// The blocks whose every completion a construct nested inside this one
    /// intercepts: sources of an exception edge into a
    /// [`total interceptor`](Self::total_interceptors) that `inside` contains.
    ///
    /// A `try` handler does not count. It selects only some completion codes,
    /// and a block whose completion may be one it does not select must keep
    /// its own exit: in `try {return $x} on error {} {exit 0} finally {set g
    /// 1}` the substitution's error reaches the handler but the `return` still
    /// runs the clause, and counting the handler edge as interception let O107
    /// empty it (found in review).
    ///
    /// With `with_handler_catches`, also the blocks a nested `try`'s
    /// unconditional handler catches whole ([`Self::handler_caught`]): an
    /// outer `finally` scan must not route those past the inner handler and
    /// clause. Loop-jump routing leaves them out — a nested `try` has already
    /// sent its own caught jumps into its handler.
    pub(in super::super) fn totally_intercepted(
        &self,
        inside: &dyn Fn(crate::cfg::BlockId) -> bool,
        with_handler_catches: bool,
    ) -> std::collections::HashSet<&str> {
        let caught = self
            .handler_caught
            .iter()
            .filter(|_| with_handler_catches)
            .filter(|block| self.block_ids.get(*block).is_some_and(|id| inside(*id)))
            .map(String::as_str);
        self.exception_edges
            .iter()
            .filter(|(_, to)| {
                self.total_interceptors.contains(to)
                    && self.block_ids.get(to).is_some_and(|id| inside(*id))
            })
            .map(|(from, _)| from.as_str())
            .chain(caught)
            .collect()
    }

    /// Whether any path through a `try`'s body or handlers reaches `end_block`
    /// by completing normally.
    ///
    /// Asked of the graph rather than of the resting tails, because a body
    /// that cannot fall through still leaves one: an inner `try` that never
    /// completes returns its unreachable `try_after_finally`, and an `if`
    /// whose every branch leaves its `if_end`. Walks the construct's own
    /// blocks from the pre-`try` block, whose exception edges lead into the
    /// handlers; anything outside the construct is not a way to `end_block`.
    fn try_completes_normally(
        &self,
        block_name: &str,
        end_block: &str,
        post_body: &str,
        body_block: &str,
        first_body_id: usize,
    ) -> bool {
        let end_id = self.bid(end_block);
        // With handlers, a body that falls through reaches `try_end` by way of
        // `try_ok`, which is allocated before the body; reaching it is normal
        // completion. Missing it made `try {set x 1} on error {} {return
        // early} finally {}` look as if it never completed, and O107 deleted
        // the code after it (found in review).
        let ok_id = self.bid(post_body);
        let body_block_id = self.bid(body_block);
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        let names: rustc_hash::FxHashMap<crate::cfg::BlockId, &str> = self
            .block_ids
            .iter()
            .map(|(name, id)| (*id, name.as_str()))
            .collect();
        let mut seen: rustc_hash::FxHashSet<&str> = rustc_hash::FxHashSet::default();
        let mut work = vec![block_name];
        while let Some(name) = work.pop() {
            if !seen.insert(name) {
                continue;
            }
            let terminator_succs = self
                .blocks
                .get(name)
                .and_then(|b| b.terminator.as_ref())
                .map_or_default(crate::cfg::Terminator::successors);
            if terminator_succs.contains(&end_id) || terminator_succs.contains(&ok_id) {
                return true;
            }
            let exception_succs = self
                .exception_edges
                .iter()
                .filter(|(from, _)| from == name)
                .filter_map(|(_, to)| self.block_ids.get(to).copied());
            for id in terminator_succs.into_iter().chain(exception_succs) {
                if in_body(id)
                    && let Some(next) = names.get(&id)
                {
                    work.push(next);
                }
            }
        }
        false
    }

    /// Bind a handler's `on`/`trap` variables at the top of its block, as the
    /// synthetic definition the ordinary walks read.
    fn push_handler_var_defs(
        &mut self,
        handler_block: &str,
        var_defs: Vec<String>,
        span: tcl_lexer::Span,
    ) {
        if var_defs.is_empty() {
            return;
        }
        self.push_statement(
            handler_block,
            Statement::Call {
                span,
                command: "try".into(),
                canonical_command: None,
                args: vec![],
                defs: var_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            },
        );
    }

    /// Lower a `finally` clause after the `try`'s end block, returning the
    /// resting block the whole statement leaves behind and the clause's own
    /// last block, if it completes normally.
    ///
    /// Only a `try` whose body or some handler can complete normally falls
    /// through the clause into the statements after it. When none can, every
    /// path into the clause is an exit, which resumes unwinding or a saved
    /// jump once the clause is done, so the clause ends as an `error` does and
    /// the code after the `try` is unreachable: `while 1 { try {break}
    /// finally {}; set x 1 }` never runs `set x 1` (found in review).
    fn lower_try_finally(
        &mut self,
        body: &crate::ir::Script,
        fin_span: Option<tcl_lexer::Span>,
        end_block: &str,
        falls_through: bool,
    ) -> (String, Option<String>) {
        let finally_block = self.new_block("try_finally");
        self.ensure_goto(end_block, &finally_block, fin_span);
        let after_finally = self.new_block("try_after_finally");
        let tail = self.lower_script(body, &finally_block);
        // A clause that itself leaves — `finally {break}` — keeps its own
        // transfer, which overrides whatever completion was pending: Tcl runs
        // the code after the loop in `while 1 { try {return} finally {break} }`.
        // Such a tail neither falls through nor resumes a saved jump
        // (found in review).
        let tail = tail.filter(|_| self.last_terminal_block.take().is_none());
        if let Some(tail) = &tail {
            if falls_through {
                self.ensure_goto(tail, &after_finally, fin_span);
            } else {
                self.set_terminator(tail, crate::cfg::Terminator::Complete {
                    route: tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt,
                    span: fin_span,
                });
                // The clause resumes unwinding, so an enclosing handler catches
                // what it raises with the clause's defs live, as for `error`.
                if let Some(blocks) = self.throw_blocks.as_mut() {
                    blocks.push(tail.clone());
                }
            }
        }
        (after_finally, tail)
    }
}

/// What lowering a `try` body leaves for its handlers and its `finally`
/// clause to be wired from.
struct TryBody {
    /// The block the body rests in, when it falls through.
    body_tail: Option<String>,
    /// The block the body ended in, when it did not fall through.
    body_terminal: Option<String>,
    /// The blocks an explicit `error` or `throw` of the body ends.
    body_throw_blocks: Vec<String>,
    /// The points a throw may leave the body from, in an analysis build.
    pub(super) split_points: Vec<super::super::SplitPoint>,
}

/// The names a `try` handler binds at the top of its block.
///
/// A `-` (fallthrough) handler has an empty body of its own and carries its
/// names on to the shared body; the target of a `-` chain may run with any
/// group member's names bound, so it defines them all.
fn handler_var_defs(
    handler: &crate::ir::TryHandler,
    pending_fallthrough_defs: &mut Vec<String>,
) -> Vec<String> {
    let own_defs: Vec<String> = handler
        .var_name
        .iter()
        .chain(&handler.options_var)
        .cloned()
        .collect();
    if handler.fallthrough {
        pending_fallthrough_defs.extend(own_defs.iter().cloned());
        return own_defs;
    }
    let mut defs = std::mem::take(pending_fallthrough_defs);
    for d in own_defs {
        if !defs.contains(&d) {
            defs.push(d);
        }
    }
    defs
}
