//! Per-command codegen hook dispatch.
//!
//! The `tcl-registry` crate exposes `CommandSpec` records with an
//! optional `codegen` hook. When a command has a hook registered,
//! this dispatcher calls it; otherwise the main emitter falls back to
//! its generic lowering path.
//!
//! Ported from `core/compiler/codegen/_bytecoded.py`.

#![allow(dead_code)]

use super::super::CodegenCtx;

/// Try to emit specialised bytecode for `cmd args...` via a
/// registered codegen hook.
///
/// Returns `true` if a hook was found and handled the command,
/// `false` if the caller should use generic lowering.
///
/// # Current status
///
/// The `tcl-registry` crate does not yet expose a codegen hook field
/// on `CommandSpec`. This is a deliberate stub until the registry-to-
/// codegen interface is finalised (tracked as a follow-up chunk).
/// When added, the implementation will look like:
///
/// ```ignore
/// pub fn try_bytecoded(ctx: &mut CodegenCtx, cmd: &str, args: &[String]) -> bool {
///     if let Some(spec) = tcl_registry::REGISTRY.get(cmd) {
///         if let Some(hook) = spec.codegen_hook {
///             return hook(ctx, args);
///         }
///     }
///     false
/// }
/// ```
pub fn try_bytecoded(_ctx: &mut CodegenCtx, _cmd: &str, _args: &[String]) -> bool {
    false
}
