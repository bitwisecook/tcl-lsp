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

//! The `Emit` seam — the target-agnostic semantic-emission interface the
//! structured walk ([`crate::codegen::structured`]) drives.
//!
//! This is Family-A of the cross-backend architecture
//! (`docs/design/common-runtime-emitter-architecture.md` §3): a backend
//! implements `Emit` to map the *structure* of a Tcl procedure onto its own
//! artifact. The first implementor is the canonical WASM pipeline's private
//! compatibility plan ([`crate::codegen::wasm::compile_wasm`]); the working bytecode emitter is
//! deliberately **not** retrofitted onto it (per the red-team: that is a rewrite,
//! not a refactor — it stays on its own driver behind the byte-identity gate).
//!
//! A backend first receives the lowered statement through
//! [`Emit::emit_typed_statement`]. Declining it preserves the original-source
//! runtime fallback through [`Emit::emit_command`].
//!
//! Control flow is **structured** — `if`/`else` and the loop scaffolding — so a
//! target with no arbitrary branches (WASM) can realise it directly. The
//! loop/function completion codes (`break`/`continue`/`return`) are their own
//! primitives; because Tcl has no labelled break they always target the
//! innermost enclosing loop, so the backend owns the (per-target) jump
//! bookkeeping and the driver need only say *what* happened, not *where to jump*.

use crate::ir::Statement;

/// The semantic operations the [structured walk](crate::codegen::structured)
/// emits. A backend maps each to its artifact (the WASM backend: an instruction
/// stream + a data section of command/expression source text).
///
/// # Loop protocol
///
/// A loop is driven by a fixed call sequence so the backend can lay out the
/// break / continue / back-edge scaffolding once:
///
/// ```text
/// begin_loop();                 // open the break + retest scopes
/// loop_test(cond);              // exit when `cond` is false (None = no guard)
/// begin_loop_body();            // open the `continue` scope
///   …body…                      // (may emit break / continue / return)
/// end_loop_body();              // close it — `continue` lands here
///   …step…                      // the `for` *next* clause (omitted otherwise)
/// end_loop();                   // back-edge + close the loop & break scopes
/// ```
///
/// A body that falls off its end re-tests via the back-edge naturally — no
/// explicit `continue` is needed for the common iteration.
pub trait Emit {
    /// Try to emit a lowered statement directly. Returning `true` means the
    /// backend consumed it; `false` keeps the original-source fallback.
    fn emit_typed_statement(&mut self, _statement: &Statement, _source: &str) -> bool {
        false
    }

    /// A leaf command, given its original source text (eval-fallback tier).
    fn emit_command(&mut self, source_text: &str);

    /// Begin an `if` whose condition is the given expression source text; the
    /// `then` region is emitted next (until the matching
    /// [`begin_else`](Emit::begin_else) / [`end_if`](Emit::end_if)).
    fn begin_if(&mut self, cond_text: &str);

    /// Begin the `else` region of the current `if`.
    fn begin_else(&mut self);

    /// End the current `if` (closes the structured region).
    fn end_if(&mut self);

    /// Open a loop: the break + retest scaffolding. Pairs with
    /// [`end_loop`](Emit::end_loop).
    fn begin_loop(&mut self);

    /// Emit the loop's continuation guard: exit the loop when `cond_text`
    /// (expression source) evaluates false. `None` = no guard — an
    /// unconditional loop, left only by `break` / `return`.
    fn loop_test(&mut self, cond_text: Option<&str>);

    /// Open the loop body — the `continue` scope. Pairs with
    /// [`end_loop_body`](Emit::end_loop_body).
    fn begin_loop_body(&mut self);

    /// Close the loop body; a `continue` lands here. Any per-iteration step (the
    /// `for` *next* clause) is emitted between this and [`end_loop`](Emit::end_loop).
    fn end_loop_body(&mut self);

    /// Emit the back-edge and close the loop + break scopes. Pairs with
    /// [`begin_loop`](Emit::begin_loop).
    fn end_loop(&mut self);

    /// `break` the innermost enclosing loop.
    fn emit_break(&mut self);

    /// `continue` the innermost enclosing loop.
    fn emit_continue(&mut self);

    /// Return from the enclosing function. The result is set by the preceding
    /// eval-fallback of the `return` command's source text.
    fn emit_return(&mut self);
}
