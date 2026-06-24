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
    /// Loop variable-binding list evaluated once before the body
    /// (`dict for {k v} …`, `dict map {k v} …`).
    LoopVarList,
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
    /// A structural keyword word — `if`'s `then`/`elseif`/`else`,
    /// `try`'s `on`/`trap`/`finally`. These sit at argument positions
    /// (not the command-name slot), so the semantic-token layer marks
    /// them with this role to highlight them as keywords rather than
    /// strings. Adding `Keyword` to a position that previously had no
    /// role is inert for every other role consumer — they filter by the
    /// roles they care about.
    Keyword,
}
