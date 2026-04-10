//! Hook type definitions for compiler integration.
//!
//! Hooks allow command-specific specialisation of lowering, codegen,
//! and constant folding. The registry stores hook **IDs** (indices
//! into dispatch tables owned by the compiler crate). This avoids
//! circular dependencies: the registry declares *what* hook, the
//! compiler implements *how*.

/// Index into the compiler's lowering hook dispatch table.
///
/// The compiler crate maintains `LOWERING_HOOKS: &[fn(...)]` and
/// uses this ID to dispatch. Commands that don't need special
/// lowering leave this as `None` on their `CommandSpec`.
pub type LoweringHookId = u16;

/// Index into the compiler's codegen hook dispatch table.
pub type CodegenHookId = u16;

/// Compile-time constant folder.
///
/// Given resolved constant argument strings, returns the computed
/// result string or `None` if the fold cannot be performed (e.g.
/// arguments are not constant, or the operation is not supported).
pub type ConstFoldFn = fn(args: &[&str]) -> Option<String>;

/// Argument type hint for a specific argument position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArgTypeHint {
    /// Expected Tcl internal representation type.
    pub expected: Option<crate::types::TclType>,
    /// Whether converting to this type destroys a previous intrep (shimmer).
    pub shimmers: bool,
}
