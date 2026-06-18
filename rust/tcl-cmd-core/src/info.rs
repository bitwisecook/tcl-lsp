//! `info` subcommand cores — the *stateful* (Family-B) half of `info`, written
//! once over a runtime's [`Introspect`] role trait + [`ValueOps`].
//!
//! Unlike the pure value cores in this crate, these reach into runtime state
//! (the call-stack), so they are generic over a type that implements **both**
//! `ValueOps` (to read the argument and build the result) and the matching
//! Family-B role trait. Both runtimes (`tcl-vm`, `runtime/rust`) satisfy the
//! bound and wrap the `Result<V, CmdError>` in their own command ABI.

use tcl_runtime_api::{Frames, Introspect, VarStore};
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `info level ?number?` — with no argument, the current call-stack depth; with
/// `number`, the command words (proc name + args) of that level. A positive
/// `number` is an absolute level; zero or negative is relative to the current
/// level (`info level 0` is the current procedure's own invocation). Mirrors
/// `Tcl_InfoLevel` / `tclCmdIL.c` `InfoLevelCmd`.
///
/// A non-integer `number` is the standard coercion error (`expected integer but
/// got "x"`); a level outside `1..=current` is `bad level "x"`.
pub fn level<O, V>(ops: &mut O, number: Option<&V>) -> Result<V, CmdError>
where
    O: ValueOps<Value = V> + Introspect<Value = V>,
{
    let cur = i64::try_from(ops.level()).unwrap_or(i64::MAX);
    let Some(n) = number else {
        return Ok(ops.new_int(cur));
    };
    // Non-integer → `expected integer but got "x"` (via `ValueError`).
    let requested = ops.as_int(n)?;
    let target = if requested <= 0 {
        cur + requested
    } else {
        requested
    };
    if target >= 1
        && let Some(argv) = ops.level_argv(usize::try_from(target).unwrap_or(0))
    {
        return Ok(argv);
    }
    // Syntactically an integer but no such call frame (out of range, or <= 0
    // at the global level): `bad level "x"`.
    Err(CmdError::new(format!("bad level \"{}\"", ops.as_str(n))))
}

/// `info exists varName` — whether `varName` is currently set in the current
/// scope: a scalar, an array, or an array element (`a(k)`). Mirrors
/// `Tcl_InfoExistsCmd` — the existence check resolves the name exactly as a read
/// would, through [`VarStore::exists`] against the current frame.
pub fn exists<O, V>(ops: &mut O, name: &V) -> V
where
    O: ValueOps<Value = V> + VarStore<Value = V> + Frames,
{
    let here = Frames::current(ops);
    let name = ops.as_str(name);
    let present = ops.exists(here, &name);
    ops.new_bool(present)
}
