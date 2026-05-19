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
    /// site.  Mirrors `core/compiler/lowering.py::_lower_proc`.
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
}

/// Typed identifier for a WASM-runtime codegen specialisation.
///
/// Reserved for the WASM-target codegen path
/// (`vm/`, the Zig WASM runtime). Currently empty — no command
/// has a WASM-specific emitter yet — but the field exists on
/// [`crate::CommandSpec`] / [`crate::SubCommand`] /
/// [`crate::forms::CommandForm`] so the per-command coverage
/// audit can track WASM hook stamping alongside the `TclVM` hook
/// without a follow-up registry refactor.
///
/// Add a variant here when a WASM-side specialisation lands;
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

/// Argument type hint for a specific argument position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArgTypeHint {
    /// Expected Tcl internal representation type.
    pub expected: Option<crate::types::TclType>,
    /// Whether converting to this type destroys a previous intrep (shimmer).
    pub shimmers: bool,
}
