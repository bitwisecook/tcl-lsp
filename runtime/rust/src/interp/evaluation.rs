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

//! Evaluating scripts: return state, sourced files, the command loop, control bodies, compiled forms, substitution, source frames and coroutines.

use super::{
    native_body_artifact, CoroContext, ErrorSnapshot, Interp, NativeCommandFrameDescription,
};
use crate::obj;
use std::rc::Rc;

mod control_bodies;
mod entry_points;
mod release_compilation;
mod return_state;
mod script_loop;
mod substitution;

/// The 1-based source line of byte `offset` in `src` — `1 + count('\n' in
/// src[0..offset])`, C's exact `TclLogCommandInfo` loop (encoding-agnostic).
pub(super) fn line_of(src: &[u8], offset: usize) -> u32 {
    1 + src[..offset.min(src.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as u32
}

/// `(compiling body of proc "NAME", line N)`: the frame `TclProcCompileProc`
/// adds where a body fails to compile, the name cut at 50 bytes.
pub(super) fn compiling_body_frame(name: &[u8], line: u32) -> Vec<u8> {
    let mut cut = name.len().min(50);
    while cut < name.len() && cut > 0 && name[cut] & 0xC0 == 0x80 {
        cut -= 1;
    }
    let mut frame = b"\n    (compiling body of proc \"".to_vec();
    frame.extend_from_slice(&name[..cut]);
    if cut < name.len() {
        frame.extend_from_slice(b"...");
    }
    frame.extend_from_slice(format!("\", line {line})").as_bytes());
    frame
}

/// Count the newlines in `s` (the line delta between two source offsets).
fn count_newlines(s: &[u8]) -> u32 {
    s.iter().filter(|&&b| b == b'\n').count() as u32
}

/// For each element of the list literal `src`, its `(newlines-before-the-element,
/// is-literal)` — the offset-aware complement to `split_list`, used to line-track
/// `{*}`-expanded literal elements (C's `TclListLines`). Returns `None` for a
/// non-UTF-8 / malformed list (the caller then falls back to body-relative).
fn scan_list_offsets(src: &[u8]) -> Option<Vec<(u32, bool)>> {
    let s = core::str::from_utf8(src).ok()?;
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        match tcl_syntax::list::find_element(s, pos) {
            Ok(Some(e)) => {
                out.push((count_newlines(&src[..e.value.start]), e.literal));
                pos = e.next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(out)
}

/// Retain the same original owners while fallible body preparation unwinds
/// before a synchronous nested evaluation. This selects no additional purpose.
enum OriginalBodyEvaluation {
    List,
    Jim,
    Compiled(Rc<native_body_artifact::NativeBodyArtifact>),
    Source(Vec<u8>),
}

struct PreparedOriginalBodyEvaluation {
    // Drop the selected artifact/source before the snapshot and original header,
    // matching the original evaluation function's local-owner order.
    pub(super) evaluation: OriginalBodyEvaluation,
    _snapshot: tcl_syntax::native_object::NativeObjectSnapshot,
    pub(super) owner: Option<obj::Owned>,
}

impl Interp {
    /// The reported `line` of the command currently executing at the top of the
    /// `info frame` stack (for fixing a source-defined proc's body line base).
    fn current_cmd_line(&self) -> u32 {
        self.cmd_frames.borrow().last().map_or(1, |f| f.line)
    }

    /// The file-absolute source line of argument `idx` of the command currently
    /// being dispatched (TIP 280); falls back to the command line. Read by a
    /// body-defining command for its body word.
    pub(crate) fn arg_line(&self, idx: usize) -> u32 {
        let lines = self.arg_lines.borrow();
        lines
            .get(idx)
            .copied()
            .unwrap_or_else(|| self.current_cmd_line())
    }

    /// Snapshot / restore the current argument lines (TIP 280) — used when a
    /// command re-dispatches a sub-slice of its own words (e.g. the
    /// single-command `oo::define <target> <sub> …` form), so the dispatched
    /// subcommand's body word is found at the right index.
    pub(crate) fn arg_lines_snapshot(&self) -> Vec<u32> {
        self.arg_lines.borrow().clone()
    }

    pub(crate) fn set_arg_lines(&self, lines: Vec<u32>) {
        *self.arg_lines.borrow_mut() = lines;
    }

    /// Mutable access to the live-coroutine registry (`cmd_coro`). Callers must
    /// drop the borrow before blocking on a coroutine handoff (the worker thread
    /// reaches the same registry to swap its context).
    pub(crate) fn coros_mut(
        &self,
    ) -> std::cell::RefMut<'_, std::collections::BTreeMap<u64, crate::cmd_coro::CoroEntry>> {
        self.coros.borrow_mut()
    }

    /// Publish a probe's original objects before acknowledging its handoff.
    /// Only the running worker accesses the interpreter until that acknowledgement.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn publish_coro_probe_error(&self, snapshot: ErrorSnapshot) {
        let previous = self.coro_probe_error.replace(Some(snapshot));
        debug_assert!(
            previous.is_none(),
            "previous probe receipt was not consumed"
        );
    }

    /// Consume the parked worker's receipt before any further coroutine handoff.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn take_coro_probe_error(&self) -> Option<ErrorSnapshot> {
        self.coro_probe_error.borrow_mut().take()
    }

    /// Swap the interp's per-flow *execution context* (call frames, the
    /// `info frame` stack, current namespace, recursion depth, return/error
    /// state, the TclOO call/define stacks, …) with `ctx`. This is how a
    /// coroutine handoff installs the resuming side's context: a single swap is
    /// its own inverse, so resume (caller→coro) and yield (coro→caller) each
    /// call it once. The shared *definitions* (namespaces, commands, classes,
    /// channels, the result object) are not swapped — coroutines share them.
    pub(crate) fn swap_coro_ctx(&self, ctx: &mut CoroContext) {
        self.native_compilation
            .borrow_mut()
            .swap_execution(&mut ctx.native_compilation);
        {
            let mut f = self.frames.borrow_mut();
            std::mem::swap(&mut *f, &mut ctx.frames);
        }
        std::mem::swap(&mut *self.cmd_frames.borrow_mut(), &mut ctx.cmd_frames);
        std::mem::swap(&mut *self.script_stack.borrow_mut(), &mut ctx.script_stack);
        std::mem::swap(&mut *self.arg_lines.borrow_mut(), &mut ctx.arg_lines);
        std::mem::swap(&mut *self.exc.borrow_mut(), &mut ctx.exc);
        std::mem::swap(&mut *self.error_stack.borrow_mut(), &mut ctx.error_stack);
        self.error_stack.borrow_mut().renew_incarnation();
        // Configuration may retire actual headers. The rest of this physical
        // handoff is mandatory even if that retirement retains the first cause.
        let stack_configuration = self.configure_native_error_stack_checked();
        std::mem::swap(
            &mut *self.jim_error_stack.borrow_mut(),
            &mut ctx.jim_error_stack,
        );
        std::mem::swap(
            &mut *self.jim_evaluation_frames.borrow_mut(),
            &mut ctx.jim_evaluation_frames,
        );
        std::mem::swap(
            &mut *self.jim_invocation_borrows.borrow_mut(),
            &mut ctx.jim_invocation_borrows,
        );
        ctx.jim_procedure_level = self.jim_procedure_level.replace(ctx.jim_procedure_level);
        ctx.native_dispatch_depth = self
            .native_dispatch_depth
            .replace(ctx.native_dispatch_depth);
        std::mem::swap(
            &mut *self.deferred_tailcalls.borrow_mut(),
            &mut ctx.deferred_tailcalls,
        );
        self.oo.borrow_mut().swap_exec(&mut ctx.oo);
        let ns = self.current_ns.replace(ctx.current_ns);
        ctx.current_ns = ns;
        let rd = self.recursion_depth.replace(ctx.recursion_depth);
        ctx.recursion_depth = rd;
        let rc = self.return_code.replace(ctx.return_code);
        ctx.return_code = rc;
        let rl = self.return_level.replace(ctx.return_level);
        ctx.return_level = rl;
        {
            let mut options = self.return_options.borrow_mut();
            std::mem::swap(&mut *options, &mut ctx.return_options);
            options.renew_incarnation();
            ctx.return_options.renew_incarnation();
        }
        std::mem::swap(
            &mut *self.array_operation_targets.borrow_mut(),
            &mut ctx.array_operation_targets,
        );
        std::mem::swap(
            &mut *self.active_var_trace_scopes.borrow_mut(),
            &mut ctx.active_var_trace_scopes,
        );
        let ed = self.eval_depth.replace(ctx.eval_depth);
        ctx.eval_depth = ed;
        let el = self.error_line.replace(ctx.error_line);
        ctx.error_line = el;
        if let Err(first) = stack_configuration {
            self.clone().refuse_native_execution(first);
        }
    }

    /// Swap the execution context of the actual coroutine generation with the live
    /// interpreter context (the resume/yield handoff in `cmd_coro`). A no-op if
    /// the coroutine is gone. The registry borrow is released before any
    /// blocking handoff.
    pub(crate) fn coro_swap_generation(&self, generation: u64) {
        // Take the context out (releasing the registry borrow), swap, put back —
        // so `swap_coro_ctx`'s RefCell touches never overlap the registry borrow.
        let taken = self
            .coros
            .borrow_mut()
            .get_mut(&generation)
            .map(|e| std::mem::replace(&mut e.context, CoroContext::placeholder()));
        if let Some(mut ctx) = taken {
            self.swap_coro_ctx(&mut ctx);
            if let Some(e) = self.coros.borrow_mut().get_mut(&generation) {
                e.context = ctx;
            }
        }
    }

    /// `info frame` (no arg): the depth of the source-location stack.
    pub(crate) fn cmd_frame_depth(&self) -> usize {
        self.cmd_frames.borrow().len()
    }

    /// The `info frame N` description for stack position `n` (C's level
    /// arithmetic: `n > 0` is absolute, 1-based from the root; `n <= 0` is
    /// relative to the current top, `0` = current). Returns the dict's
    /// (key, value) pairs in C's key order (`type line [file] cmd [proc]
    /// level`), or `None` if out of range.
    pub(crate) fn cmd_frame_info(
        &mut self,
        n: i64,
    ) -> Result<Option<NativeCommandFrameDescription>, tcl_syntax::value::ValueError> {
        let original = {
            let frames = self.cmd_frames.borrow();
            let depth = frames.len() as i64;
            let pos = if n > 0 { n } else { depth + n };
            if pos < 1 || pos > depth {
                return Ok(None);
            }
            frames[(pos - 1) as usize].original_command.clone()
        };
        let original_bytes = match original {
            Some(value) => Some(self.eval_object_bytes(value.as_ptr())?),
            None => None,
        };
        let cmd_frames = self.cmd_frames.borrow();
        let depth = cmd_frames.len() as i64;
        let pos = if n > 0 { n } else { depth + n };
        if pos < 1 || pos > depth {
            return Ok(None);
        }
        let f = &cmd_frames[(pos - 1) as usize];
        let mut pairs = vec![
            (b"type".to_vec(), f.kind.as_bytes().to_vec()),
            (b"line".to_vec(), f.line.to_string().into_bytes()),
        ];
        if let Some(file) = &f.file {
            pairs.push((b"file".to_vec(), file.to_vec()));
        }
        pairs.push((
            b"cmd".to_vec(),
            original_bytes
                .as_ref()
                .map_or_else(|| f.cmd.clone(), |bytes| bytes.to_vec()),
        ));
        // A TclOO method frame reports `method`/`class`|`object` (the declarer),
        // and an `apply` lambda reports `lambda <expr>`, in place of `proc`
        // (C's `TclInfoFrame`).
        if let Some((method, kind, owner)) = &f.oo {
            if !method.is_empty() {
                pairs.push((b"method".to_vec(), method.clone()));
            }
            pairs.push((kind.clone(), owner.clone()));
        } else if let Some(l) = &f.lambda {
            pairs.push((b"lambda".to_vec(), l.clone()));
        } else if let Some(p) = &f.proc {
            pairs.push((b"proc".to_vec(), p.clone()));
        }
        // `level` is the distance from the current call level — but C only adds
        // it when the frame's CallFrame is *reachable* from the current var frame
        // by walking the caller chain (`TclInfoFrame`). A frame bypassed by an
        // `uplevel` redirection (e.g. the proc that called `uplevel`, when viewed
        // from the uplevel'd callee) is off the chain and omits `level`, even
        // though it shares a level with a chain frame. `omit_level` short-circuits
        // the explicit `uplevel`-body case (C's `framePtr->framePtr == NULL`).
        if !f.omit_level {
            let frames = self.frames.borrow();
            if frames.caller_chain_indices().contains(&f.frame_index) {
                let level = frames.current_level().saturating_sub(f.level);
                pairs.push((b"level".to_vec(), level.to_string().into_bytes()));
            }
        }
        Ok(Some(pairs))
    }
}
