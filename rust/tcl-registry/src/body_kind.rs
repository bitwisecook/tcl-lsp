//! Body-kind classification for `ArgRole::Body` arguments.
//!
//! Mirrors Python's `core/commands/registry/body_kind.py::BodyKind`
//! (introduced in `88970edc` / `91daf5c2`, closes `#250`).  Tells SSA
//! and other data-flow consumers whether a body argument shares the
//! caller's frame (`Plain`) or runs in a definition / dispatch context
//! that is *not* the caller's scope (`Structural`).
//!
//! ## Why
//!
//! `proc`, `oo::class create`, `oo::define`'s script-bearing
//! subcommands, `snit::method`, `snit::typemethod`, and
//! `uri::register` all carry a body argument that is *not* executed
//! in the caller's frame.  Without this distinction SSA would scan
//! variable references inside a method body as reads/writes against
//! the surrounding scope — producing false def-use edges and bogus
//! diagnostics.
//!
//! `if`, `while`, `for`, `foreach`, `catch`, `try`, … bodies *do*
//! share the caller's frame, so the default `Plain` is correct for
//! every command that doesn't opt into `Structural`.

/// Whether a body argument runs in the caller's frame
/// ([`Plain`](BodyKind::Plain)) or in a separate scope
/// ([`Structural`](BodyKind::Structural)).
///
/// Default is `Plain` so existing specs don't need touching when the
/// field is added.  Stamp `Structural` only on commands whose body is
/// known to run in a definition / dispatch context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum BodyKind {
    /// Body runs in the caller's frame.  Variable references inside
    /// the body resolve against the enclosing scope; SSA scans them
    /// as part of the surrounding block's data flow.
    #[default]
    Plain,
    /// Body runs in a definition or dispatch context that is not the
    /// caller's scope.  SSA must skip the body when scanning the
    /// enclosing block (the body still gets analysed as its own
    /// scope by the OO / proc / snit / uri analyser pieces).
    Structural,
}
