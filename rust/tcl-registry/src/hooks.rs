//! Hook type definitions for compiler integration.
//!
//! Hooks let command-specific specialisation slot into lowering and
//! codegen without baking command names into the compiler. The
//! registry stores a typed identifier on each [`crate::CommandSpec`]
//! / [`crate::SubCommand`]; the compiler maps that identifier to its
//! algorithm. Identifiers are exhaustive enums so a new compiler
//! pass cannot accidentally accept an arbitrary integer.

/// Typed identifier for a lowering specialisation.
///
/// The compiler keeps the implementations; the registry keeps the
/// catalogue of which command form picks which implementation.
/// Variants are stable enum members rather than bare integers so a
/// `match` on this type is exhaustively checked at every dispatcher
/// — adding a new hook here gives the compiler a deliberate
/// compile-time error until the new arm is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoweringHookId {
    /// `expr <single-arg>` → typed expression IR.
    Expr,
    /// `return ?value?` with non-option, non-expanded args.
    Return,
    /// `set name value` → typed assignment IR.
    Set,
    /// `incr name ?amount?` → typed increment IR.
    Incr,
    /// `append` / `lappend name value...` — variable read-write.
    AppendOrLappend,
    /// `unset ?-nocomplain? ?--? name...`.
    Unset,
    /// `global name...`.
    Global,
    /// `variable name ?value?...`.
    Variable,
    /// `upvar ?level? otherVar localVar ...`.
    Upvar,
}

/// Typed identifier for a bytecoded codegen specialisation.
///
/// The compiler's codegen layer holds the per-variant emitter. Keep
/// this enum in sync with the dispatch table in
/// [`tcl_compiler::codegen::emitter::bytecoded`]; a new variant here
/// gives the compiler a compile-time match-exhaustion error until
/// the new arm is wired up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenHookId {
    /// `lassign list var1 ?var2 ...?`.
    Lassign,
    /// `llength list`.
    Llength,
    /// `lrange list first last`.
    Lrange,
    /// `linsert list index element ?element ...?`.
    Linsert,
    /// `lset varname ?index ...? value`.
    Lset,
    /// `dict <subcommand> ...`.
    Dict,
    /// `array <subcommand> ...`.
    Array,
}

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
