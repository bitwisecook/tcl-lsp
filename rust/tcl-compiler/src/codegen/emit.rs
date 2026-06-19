//! The `Emit` seam — the target-agnostic semantic-emission interface the
//! structured walk ([`crate::codegen::structured`]) drives.
//!
//! This is Family-A of the cross-backend architecture
//! (`docs/design/common-runtime-emitter-architecture.md` §3): a backend
//! implements `Emit` to map the *structure* of a Tcl procedure onto its own
//! artifact. The first implementor is the greenfield WASM backend
//! ([`crate::codegen::wasm::backend`]); the working bytecode emitter is
//! deliberately **not** retrofitted onto it (per the red-team: that is a rewrite,
//! not a refactor — it stays on its own driver behind the byte-identity gate).
//!
//! The statement tier is **eval-fallback**: every leaf command is handed to the
//! backend as its original source text, to be evaluated by the runtime at run
//! time (`tcl_eval`). The inline AOT tiers (variable slots, arithmetic, …) and
//! the per-command codegen hooks are a later stage, behind the same seam.
//!
//! Control flow is **structured** — `if`/`else` and the loop scaffolding — so a
//! target with no arbitrary branches (WASM) can realise it directly. The
//! loop/function completion codes (`break`/`continue`/`return`) are their own
//! primitives; because Tcl has no labelled break they always target the
//! innermost enclosing loop, so the backend owns the (per-target) jump
//! bookkeeping and the driver need only say *what* happened, not *where to jump*.

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
