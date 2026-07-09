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
    /// A command prefix — a partial command whose first word is a command
    /// name, invoked at runtime with further arguments appended (`lsort
    /// -command cmdPrefix`, trace callbacks).  Distinct from `Body` (a
    /// complete script to recurse) and from a generic `Value`: the first word
    /// is a callable reference, not a script — marking such a value `Body`
    /// would wrongly recurse a bareword proc name as a script.
    ///
    /// A first-class command **reference**: the compiler records the prefix
    /// head as a call site (highlighting, find-references, call graph,
    /// call-hierarchy, dead-code, W123) and — via the paired
    /// [`AppendedArity`] — checks the callback's arity.  The number of args
    /// the calling command appends lives in the registry beside the role
    /// (`CommandSpec::command_prefixes` / `command_prefix_resolver`, or an
    /// option's [`crate::hover::OptionArg::appended_arity`]), never in the
    /// compiler.
    CommandPrefix,
}

/// How many arguments a command appends to a [`ArgRole::CommandPrefix`]
/// callback when it invokes it.
///
/// Sourced from C Tcl 9.0 behaviour (`lsort -command` appends 2, `trace add
/// variable` appends 3, `socket -server` appends 3, …).  Paired with a
/// `CommandPrefix` declaration so the arity checker can validate that the
/// referenced proc accepts `baked_args + appended` arguments, where
/// `baked_args` are any words already present in the prefix itself
/// (`{myCmp extra}` bakes one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AppendedArity {
    /// Exactly `n` args are appended (`lsort -command` → `Exactly(2)`).
    Exactly(u8),
    /// At least `n` args are appended; the maximum is unbounded or
    /// form-dependent (`trace add execution` → `AtLeast(2)`, `regsub
    /// -command` → `AtLeast(1)`, variadic `interp alias` → `AtLeast(0)`).
    AtLeast(u8),
    /// The appended count can't be determined statically — no arity check.
    /// The default, so a bare `CommandPrefix` declaration is arity-inert.
    #[default]
    Unknown,
}

impl AppendedArity {
    /// The minimum number of appended args (0 when [`Unknown`](Self::Unknown)).
    #[must_use]
    pub const fn min(self) -> u8 {
        match self {
            Self::Exactly(n) | Self::AtLeast(n) => n,
            Self::Unknown => 0,
        }
    }

    /// The maximum number of appended args, or `None` when unbounded /
    /// unknown.
    #[must_use]
    pub const fn max(self) -> Option<u8> {
        match self {
            Self::Exactly(n) => Some(n),
            Self::AtLeast(_) | Self::Unknown => None,
        }
    }

    /// Whether an arity check should run at all — `false` for
    /// [`Unknown`](Self::Unknown), whose count is indeterminate.
    #[must_use]
    pub const fn is_checkable(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}
