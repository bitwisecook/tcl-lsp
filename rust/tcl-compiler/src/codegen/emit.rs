//! The `Emit` seam — the target-agnostic semantic-emission interface the
//! structured CFG walk ([`crate::codegen::cfg_walk`]) drives.
//!
//! This is Family-A of the cross-backend architecture
//! (`docs/design/common-runtime-emitter-architecture.md` §3): a backend
//! implements `Emit` to map the *structure* of a Tcl procedure (recovered from
//! the CFG) onto its own artifact. The first implementor is the greenfield WASM
//! backend ([`crate::codegen::wasm::backend`]); the working bytecode emitter is
//! deliberately **not** retrofitted onto it (per the red-team: that is a rewrite,
//! not a refactor — it stays on its own driver behind the byte-identity gate).
//!
//! Stage 1's statement tier is **eval-fallback**: every leaf command is handed
//! to the backend as its original source text, to be evaluated by the runtime at
//! run time (`tcl_eval`). The inline AOT tiers (variable slots, arithmetic, …)
//! and the per-command codegen hooks are Stage 2, behind the same seam.

/// The semantic operations the structured CFG walk emits. A backend maps each
/// to its artifact (the WASM backend: instruction stream + data section).
///
/// Control flow is **structured** (`if`/`else`/loops) rather than address jumps,
/// because WASM has no arbitrary branches — the [`crate::codegen::cfg_walk`]
/// driver reconstructs structured regions from the Tcl-shaped (always reducible)
/// CFG and calls these in nesting order.
pub trait Emit {
    /// A leaf command, given its original source text (eval-fallback tier).
    fn emit_command(&mut self, source_text: &str);

    /// Begin an `if` whose condition is the given source text; the `then` region
    /// is emitted next (until the matching [`Emit::begin_else`]/[`Emit::end_if`]).
    fn begin_if(&mut self, cond_text: &str);

    /// Begin the `else` region of the current `if`.
    fn begin_else(&mut self);

    /// End the current `if` (closes the structured region).
    fn end_if(&mut self);
}
