//! Argument roles — what role each argument plays in a command.

/// What role an argument plays in a command invocation.
///
/// Used by the compiler, analyser, and LSP features to understand
/// which arguments are scripts to recurse into, which are variable
/// names, which are expressions, etc. Consumers query roles via the
/// registry — never by matching on command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ArgRole {
    /// Tcl script body — recursively analysed.
    Body,
    /// Expression (`expr` sub-language).
    Expr,
    /// Variable name written by the command (`set`, `incr`, `lassign`).
    VarWrite,
    /// Variable name read without modification (`info exists`, `array get`).
    VarRead,
    /// Procedure parameter list.
    ParamList,
    /// Symbolic name (proc name, namespace name).
    Name,
    /// Pattern or regex.
    Pattern,
    /// Switch/flag option.
    Option,
    /// Generic value argument.
    Value,
    /// The subcommand word (e.g. `"length"` in `string length`).
    Subcommand,
    /// The `--` option terminator.
    OptionTerminator,
    /// Channel identifier (`stdout`, `stdin`, channel ID).
    Channel,
    /// List/string index expression.
    Index,
}
