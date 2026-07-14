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
    /// `proc name params body` — defines a procedure.  Lowered to a
    /// nested IR script + `Statement::Call` at the proc declaration
    /// site.
    Proc,
    /// `when EVENT ?priority N? body` — iRules event handler.
    /// Lowered the same shape as `proc` but indexed by event name.
    When,
    /// `namespace eval ns body` — runs the body in a separate
    /// namespace scope.
    NamespaceEval,
    /// `if cond body ?elseif cond body ...? ?else body?` — typed
    /// conditional with `IfClause` arms + optional else body.
    If,
    /// `switch ?options? subject pattern body ...` — typed multi-
    /// arm dispatch.
    Switch,
    /// `for init cond next body` — typed loop with init / cond /
    /// next / body scripts.
    For,
    /// `while cond body` — typed loop.
    While,
    /// `foreach varList listExpr body` — typed loop with iterator
    /// groups.
    Foreach,
    /// `lmap varList listExpr body` — like `foreach` but collects
    /// each iteration's body result into a list.
    Lmap,
    /// `foreachLine varName filename body` — Tcl 9.0 file-iteration
    /// loop (TIP 670).  Distinct from `Foreach` because the body
    /// runs against file lines, not a literal list — the lowerer
    /// uses a dedicated emitter (`lower_foreach_line`) that
    /// requires `&mut self` for proc-depth / const-map state.
    ForeachLine,
    /// `catch body ?resultVarName? ?optionsVarName?` — typed
    /// exception barrier.
    Catch,
    /// `try body ?on/trap handlers? ?finally body?` — typed try
    /// with handler clauses + optional finally.
    Try,
    /// `dict <subcommand> ...` — dispatches to a per-subcommand
    /// emitter (see `CodegenHookId::Dict` for the codegen side).
    Dict,
    /// `eval ?arg ...?` — runtime barrier with optional static-body
    /// relaxation when the body is a brace-literal that passes the
    /// `body_has_dynamic_barrier` gate.
    Eval,
    /// `uplevel ?level? ?arg ...?` — runtime barrier with optional
    /// static-body relaxation under the same gate.
    Uplevel,
    /// `apply {{params} body ?ns?} ?arg ...?` — an anonymous lambda.
    /// Runs its body in a separate frame, so the call stays a runtime
    /// barrier; but a braced literal body is still walked (in a fresh
    /// frame) so nested `proc` definitions and other module-level
    /// effects register — like `NamespaceEval`, not the fully opaque
    /// default barrier.
    Apply,
    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0) — iterates
    /// the array's entries. Like `Apply`, the call stays a runtime barrier
    /// (C Tcl compiles it to an `invokeStk` of `::tcl::array::for` with the
    /// body pushed as an unparsed literal — it does **not** compile the body),
    /// but a braced literal body is walked in a fresh frame bound to the two
    /// loop variables so nested definitions register and the body is analysable.
    ArrayFor,
}

/// Typed identifier for a `TclVM` bytecode codegen specialisation.
///
/// Identifies which command shape gets a hand-written
/// emitter in the compiler's `TclVM` bytecode layer (the path that
/// matches C Tcl 9's `bytecode` output). The compiler's codegen
/// layer holds the per-variant emitter. Keep this enum in sync
/// with the dispatch table in
/// `tcl_compiler::codegen::emitter::bytecoded`; a new variant
/// here gives the compiler a compile-time match-exhaustion error
/// until the new arm is wired up.
///
/// **Note:** despite the historical name `CodegenHookId`, this
/// covers only the `TclVM` bytecode emitter. The WASM emitter
/// family has its own [`WasmCodegenHookId`].
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
    /// `namespace <subcommand> ...` — the `eval` form compiles to the
    /// ensemble-rewrite `invokeReplace` of `::tcl::namespace::eval`.
    Namespace,
    /// `append varName ?value ...?`.
    Append,
    /// `lappend varName ?value ...?`.
    Lappend,
    /// `unset ?-nocomplain? ?--? ?varName ...?`.
    Unset,
    /// `tailcall command ?arg ...?`.
    Tailcall,
    /// `concat ?arg ...?`.
    Concat,
    /// `global ?varName ...?`.
    Global,
    /// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
    Upvar,
}

/// Typed identifier for a WASM-runtime codegen specialisation.
///
/// Reserved for the WASM-target codegen path
/// (`vm/`, the WASM runtime). Currently empty — no command
/// has a WASM-specific emitter yet — but the field exists on
/// [`crate::CommandSpec`] / [`crate::SubCommand`] /
/// [`crate::forms::CommandForm`] so the per-command coverage
/// audit can track WASM hook stamping alongside the `TclVM` hook.
///
/// Add a variant here when a WASM-side specialisation is added;
/// keep it in sync with whatever dispatcher the WASM emitter
/// uses on the compiler side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmCodegenHookId {}

/// Compile-time constant folder.
///
/// Given resolved constant argument strings, returns the computed
/// result string or `None` if the fold cannot be performed (e.g.
/// arguments are not constant, or the operation is not supported).
pub type ConstFoldFn = fn(args: &[&str]) -> Option<String>;

/// A specific Tcl release whose **compile-time** semantics a constant fold may
/// depend on — e.g. `string is integer` is unbounded on 9.0 but caps at
/// `2³²-1` on 8.x, and `string is wideinteger` / `entier` / `dict` and
/// `format %b` don't exist (they *raise*) before a given release.  Ordered, so
/// a fold can test `version >= TclVersion::V8_5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TclVersion {
    /// Tcl 8.4.
    V8_4,
    /// Tcl 8.5.
    V8_5,
    /// Tcl 8.6.
    V8_6,
    /// Tcl 9.0.
    V9_0,
    /// Tcl 9.1. Shares 9.0's compile-time fold semantics (a superset release);
    /// ordered after `V9_0` so `>= V9_0` gates include it.
    V9_1,
}

impl TclVersion {
    /// Map an optimiser dialect string (`"tcl8.4"` … `"tcl9.1"`) to a version,
    /// or `None` for an unversioned (`"tcl"`), non-Tcl (`"f5-irules"`), or
    /// unknown dialect — in which case a versioned fold must return only the
    /// dialect-invariant subset every release shares.
    #[must_use]
    pub fn from_dialect(dialect: Option<&str>) -> Option<Self> {
        match dialect {
            Some("tcl8.4") => Some(Self::V8_4),
            Some("tcl8.5") => Some(Self::V8_5),
            Some("tcl8.6") => Some(Self::V8_6),
            Some("tcl9.0") => Some(Self::V9_0),
            // 9.1 must not fall through to `None` (which degrades a versioned
            // fold to the dialect-invariant subset) — it behaves as 9.0+
            // (RUST_ISSUE_083).
            Some("tcl9.1") => Some(Self::V9_1),
            _ => None,
        }
    }
}

/// A Tcl-version-aware constant folder.  `version` is `None` when the target
/// dialect doesn't name a specific Tcl release, in which case the fold must
/// return only the dialect-invariant result every version agrees on (or
/// `None`).  Used for commands whose compile-time value depends on the Tcl
/// version (`string is`, `format`, `scan`).
pub type VersionedConstFoldFn = fn(args: &[&str], version: Option<TclVersion>) -> Option<String>;

/// Argument type hint for a specific argument position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArgTypeHint {
    /// Expected Tcl internal representation type.
    pub expected: Option<crate::types::TclType>,
    /// Whether converting to this type destroys a previous intrep (shimmer).
    pub shimmers: bool,
    /// Current intreps the operation reads directly, without installing
    /// [`Self::expected`] — no shimmer happens for an operand already in one
    /// of these representations even though it differs from `expected`.
    /// E.g. `string length`/`index`/`range` have a pure-byte-array fast path
    /// (`Tcl_GetCharLength` short-circuits before `SetStringFromAny`;
    /// tclsh-verified the intrep survives), so their string argument is
    /// `expected: String, shimmers: true, transparent_from: &[ByteArray]`.
    pub transparent_from: &'static [crate::types::TclType],
}

#[cfg(test)]
mod tests {
    use super::TclVersion;

    #[test]
    fn from_dialect_maps_every_versioned_tcl() {
        // RUST_ISSUE_083: tcl9.1 must resolve to a version, not `None` (which
        // silently degrades versioned folds to the dialect-invariant subset).
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.4")),
            Some(TclVersion::V8_4)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.5")),
            Some(TclVersion::V8_5)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.6")),
            Some(TclVersion::V8_6)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.0")),
            Some(TclVersion::V9_0)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.1")),
            Some(TclVersion::V9_1)
        );
        // Unversioned / non-Tcl / unknown → None.
        assert_eq!(TclVersion::from_dialect(Some("tcl")), None);
        assert_eq!(TclVersion::from_dialect(Some("f5-irules")), None);
        assert_eq!(TclVersion::from_dialect(None), None);
    }

    #[test]
    fn v9_1_orders_at_or_above_v9_0() {
        // A `>= V9_0` gate must include 9.1.
        assert!(TclVersion::V9_1 >= TclVersion::V9_0);
        assert!(TclVersion::V9_1 > TclVersion::V8_6);
    }
}
