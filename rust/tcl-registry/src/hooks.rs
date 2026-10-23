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

macro_rules! source_catalogue_enum {
    ($(#[$attribute:meta])* pub enum $name:ident {
        $($(#[$variant_attribute:meta])* $variant:ident => ($order:expr, $summary:expr)),* $(,)?
    }) => {
        $(#[$attribute])* pub enum $name {
            $($(#[$variant_attribute])* $variant),*
        }
        impl $name {
            /// Stable authored source picker vocabulary, in presentation order.
            pub const SOURCE_PRESENTATION: &'static [(&'static str, &'static str)] = &{
                let mut rows = [("", ""); 0 $(+ { let _ = stringify!($variant); 1 })*];
                $(rows[$order] = (stringify!($variant), $summary);)*
                rows
            };
        }
    };
}
pub(crate) use source_catalogue_enum;

source_catalogue_enum! {
/// Typed identifier for a lowering specialisation.
///
/// The compiler keeps the implementations; the registry keeps the
/// catalogue of which command form picks which implementation.
/// Variants are stable enum members rather than bare integers so a
/// `match` on this type is exhaustively checked at every dispatcher
/// — adding a new hook here gives the compiler a deliberate
/// compile-time error until the new arm is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LoweringHookId {
    /// `expr <single-arg>` → typed expression IR.
    Expr => (0, "expr"),
    /// `return ?value?` with non-option, non-expanded args.
    Return => (1, "return"),
    /// `set name value` → typed assignment IR.
    Set => (2, "set"),
    /// `incr name ?amount?` → typed increment IR.
    Incr => (3, "incr"),
    /// `append` / `lappend name value...` — variable read-write.
    AppendOrLappend => (4, "append / lappend"),
    /// `unset ?-nocomplain? ?--? name...`.
    Unset => (5, "unset"),
    /// `global name...`.
    Global => (6, "global"),
    /// `variable name ?value?...`.
    Variable => (7, "variable"),
    /// `upvar ?level? otherVar localVar ...`.
    Upvar => (8, "upvar"),
    /// `proc name params body` — defines a procedure.  Lowered to a
    /// nested IR script + `Statement::Call` at the proc declaration
    /// site.
    Proc => (9, "proc"),
    /// `when EVENT ?priority N? body` — iRules event handler.
    /// Lowered the same shape as `proc` but indexed by event name.
    When => (10, "iRules when"),
    /// `namespace eval ns body` — runs the body in a separate
    /// namespace scope.
    NamespaceEval => (11, "namespace eval"),
    /// `if cond body ?elseif cond body ...? ?else body?` — typed
    /// conditional with `IfClause` arms + optional else body.
    If => (12, "if"),
    /// `switch ?options? subject pattern body ...` — typed multi-
    /// arm dispatch.
    Switch => (13, "switch"),
    /// `for init cond next body` — typed loop with init / cond /
    /// next / body scripts.
    For => (14, "for"),
    /// `while cond body` — typed loop.
    While => (15, "while"),
    /// `foreach varList listExpr body` — typed loop with iterator
    /// groups.
    Foreach => (16, "foreach"),
    /// `lmap varList listExpr body` — like `foreach` but collects
    /// each iteration's body result into a list.
    Lmap => (17, "lmap"),
    /// `foreachLine varName filename body` — Tcl 9.0 file-iteration
    /// loop (TIP 670).  Distinct from `Foreach` because the body
    /// runs against file lines, not a literal list — the lowerer
    /// uses a dedicated emitter (`lower_foreach_line`) that
    /// requires `&mut self` for proc-depth / const-map state.
    ForeachLine => (18, "foreach-line reader idiom"),
    /// `catch body ?resultVarName? ?optionsVarName?` — typed
    /// exception barrier.
    Catch => (19, "catch"),
    /// `try body ?on/trap handlers? ?finally body?` — typed try
    /// with handler clauses + optional finally.
    Try => (20, "try"),
    /// `dict <subcommand> ...` — dispatches to a per-subcommand
    /// emitter (see `CodegenHookId::Dict` for the codegen side).
    Dict => (21, "dict"),
    /// `eval ?arg ...?` — runtime barrier with optional static-body
    /// relaxation when the body is a brace-literal that passes the
    /// `body_has_dynamic_barrier` gate.
    Eval => (22, "eval"),
    /// `uplevel ?level? ?arg ...?` — runtime barrier with optional
    /// static-body relaxation under the same gate.
    Uplevel => (23, "uplevel"),
    /// `apply {{params} body ?ns?} ?arg ...?` — an anonymous lambda.
    /// Runs its body in a separate frame, so the call stays a runtime
    /// barrier; but a braced literal body is still walked (in a fresh
    /// frame) so nested `proc` definitions and other module-level
    /// effects register — like `NamespaceEval`, not the fully opaque
    /// default barrier.
    Apply => (24, "apply"),
    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0) — iterates
    /// the array's entries. Like `Apply`, the call stays a runtime barrier
    /// (C Tcl compiles it to an `invokeStk` of `::tcl::array::for` with the
    /// body pushed as an unparsed literal — it does **not** compile the body),
    /// but a braced literal body is walked in a fresh frame bound to the two
    /// loop variables so nested definitions register and the body is analysable.
    ArrayFor => (25, "array for"),
}
}

impl LoweringHookId {
    /// Stable compiler and Explorer spelling for this structural identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expr => "expr",
            Self::Return => "return",
            Self::Set => "set",
            Self::Incr => "incr",
            Self::AppendOrLappend => "append-or-lappend",
            Self::Unset => "unset",
            Self::Global => "global",
            Self::Variable => "variable",
            Self::Upvar => "upvar",
            Self::Proc => "proc",
            Self::When => "when",
            Self::NamespaceEval => "namespace-eval",
            Self::If => "if",
            Self::Switch => "switch",
            Self::For => "for",
            Self::While => "while",
            Self::Foreach => "foreach",
            Self::Lmap => "lmap",
            Self::ForeachLine => "foreach-line",
            Self::Catch => "catch",
            Self::Try => "try",
            Self::Dict => "dict",
            Self::Eval => "eval",
            Self::Uplevel => "uplevel",
            Self::Apply => "apply",
            Self::ArrayFor => "array-for",
        }
    }
}

source_catalogue_enum! {
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
/// covers only the `TclVM` bytecode emitter. Other compiler backends
/// keep their specialisation registries with their emitters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenHookId {
    /// `lassign list var1 ?var2 ...?`.
    Lassign => (0, "lassign"),
    /// `llength list`.
    Llength => (1, "llength"),
    /// `lrange list first last`.
    Lrange => (2, "lrange"),
    /// `linsert list index element ?element ...?`.
    Linsert => (3, "linsert"),
    /// `lset varname ?index ...? value`.
    Lset => (4, "lset"),
    /// `dict <subcommand> ...`.
    Dict => (5, "dict"),
    /// `array <subcommand> ...`.
    Array => (6, "array"),
    /// `namespace <subcommand> ...` — the `eval` form compiles to the
    /// ensemble-rewrite `invokeReplace` of `::tcl::namespace::eval`.
    Namespace => (7, "namespace"),
    /// `append varName ?value ...?`.
    Append => (8, "append"),
    /// `lappend varName ?value ...?`.
    Lappend => (9, "lappend"),
    /// `unset ?-nocomplain? ?--? ?varName ...?`.
    Unset => (10, "unset"),
    /// `tailcall command ?arg ...?`.
    Tailcall => (11, "tailcall"),
    /// `concat ?arg ...?`.
    Concat => (12, "concat"),
    /// `global ?varName ...?`.
    Global => (13, "global"),
    /// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
    Upvar => (14, "upvar"),
    /// Selected C Tcl 9.1 `uplevel` frame-evaluation operation.
    Uplevel => (15, "uplevel"),
    /// `set varName` — the one-word form, a load of the variable
    /// (`TclCompileSetCmd`).
    Set => (16, "set"),
    /// `array set arrayName list` — the command runs, its array a compiled
    /// local (`TclCompileArraySetCmd`; subcommand-keyed).
    ArraySet => (17, "array set"),
    /// `array unset arrayName` — the command runs, its array a compiled
    /// local; with a pattern `TclCompileArrayUnsetCmd` makes none
    /// (subcommand-keyed).
    ArrayUnset => (18, "array unset"),
    /// `variable name ?value name value ...?` — the command runs, the tail of
    /// each name a compiled local (`TclCompileVariableCmd`).
    Variable => (19, "variable"),
    /// `namespace upvar ns otherVar myVar ...` — the command runs, each
    /// `myVar` a compiled local (`TclCompileNamespaceUpvarCmd`;
    /// subcommand-keyed).
    NamespaceUpvar => (20, "namespace upvar"),
}
}

source_catalogue_enum! {
/// Typed identifier for an *inline* (value-position) bytecode codegen
/// specialisation.
///
/// Identifies which command shape gets a hand-written inline emitter
/// on the compiler's command-substitution / catch-body paths — the
/// `[cmd …]` value-position dispatch in
/// `tcl_compiler::codegen::cmd_subst::emit_inline_cmd_subst` and the
/// single-command catch/try-body dispatch in
/// `tcl_compiler::codegen::control_flow::emit_catch_body`. The
/// compiler owns the per-variant emitters (and their applicability
/// guards — arity / shape / proc-context); the registry stamps which
/// command form picks which emitter, exactly as [`CodegenHookId`]
/// does for the statement-position dispatch in
/// `tcl_compiler::codegen::emitter::bytecoded`. A new variant here
/// gives the compiler a compile-time match-exhaustion error until the
/// new arm is wired up. A dispatcher matches only the variants it
/// specialises in its own context — an unmatched variant falls to
/// that dispatcher's generic-invoke arm (e.g. [`Self::Break`] is
/// specialised in a catch body but generic in plain value position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineCodegenHookId {
    /// `expr expression` — inline typed-expression compilation
    /// (value position and catch body).
    Expr => (0, "expr"),
    /// `incr varName ?increment?` — LVT / stack increment.
    Incr => (1, "incr"),
    /// `set varName` — the one-word form, a load of the variable
    /// (`TclCompileSetCmd`).
    Set => (25, "set"),
    /// `info exists varName` — `existScalar` / `existStk`
    /// (subcommand-keyed: stamped on `info`'s `exists` subcommand).
    InfoExists => (2, "info exists"),
    /// Native absolute trivial `info commands` compiler operation.
    InfoCommandsResolve => (7, "info commands absolute literal"),
    /// Native stack-level introspection.
    InfoLevel => (3, "info level"),
    /// Return the namespace of the currently executing native activation.
    NamespaceCurrent => (4, "namespace current"),
    /// Resolve a command's imported origin after evaluating its name operand.
    NamespaceOrigin => (5, "namespace origin name"),
    /// Native literal namespace-scoped prefix builder.
    NamespaceCode => (6, "namespace code literal-script"),
    /// `string <subcommand> …` — per-subcommand string ops with an
    /// FQN `::tcl::string::*` invoke fallback.
    String => (8, "string"),
    /// `lindex list index ?index …?`.
    Lindex => (9, "lindex"),
    /// `lrange list first last`.
    Lrange => (10, "lrange"),
    /// `lreplace list first last ?element …?`.
    Lreplace => (11, "lreplace"),
    /// `linsert list index ?element …?`.
    Linsert => (12, "linsert"),
    /// `regexp ?-nocase? ?--? pattern subject`.
    Regexp => (13, "regexp"),
    /// `list ?arg …?` (non-expanding form).
    List => (14, "list"),
    /// `array <subcommand> …` — `exists` / `names` / `size` forms.
    Array => (15, "array"),
    /// `dict get dictionary ?key …?` (subcommand-keyed: stamped on
    /// `dict`'s `get` subcommand).
    DictGet => (16, "dict get"),
    /// `catch body ?resultVar? ?optionsVar?` — inline
    /// `beginCatch4`/`endCatch` sequence.
    Catch => (17, "catch"),
    /// `return ?-code C? ?-level L? ?value?` inside a catch body —
    /// `returnImm` with compile-time-folded code/level.
    Return => (18, "return"),
    /// `error message ?info? ?code?` inside a catch/try body —
    /// `returnImm 1 0`.
    Error => (19, "error"),
    /// `break` inside a catch body — the `break` opcode.
    Break => (20, "break"),
    /// `continue` inside a catch body — the `continue` opcode.
    Continue => (21, "continue"),
    /// Suspend the current coroutine with zero or one produced value.
    Yield => (23, "yield"),
    /// Suspend and relay an invocation in the coroutine's current namespace.
    YieldTo => (24, "yieldto"),
    /// `try body on error {var} handler` inside a catch body —
    /// inline two-range catch/handler sequence.
    Try => (22, "try"),
    /// `string is class ?-strict? string` — `strclass` and the numeric
    /// checks `TclCompileStringIsCmd` compiles (subcommand-keyed).
    StringIs => (26, "string is"),
    /// `string match ?-nocase? pattern string` — `streq` for a literal
    /// pattern with no glob character, else `strmatch`
    /// (`TclCompileStringMatchCmd`; subcommand-keyed).
    StringMatch => (27, "string match"),
    /// `string replace string first last ?newString?` — the forms
    /// `TclCompileStringReplaceCmd` settles on literal indices, else
    /// `strreplace` (subcommand-keyed).
    StringReplace => (28, "string replace"),
    /// `string trim string ?chars?` — `strtrim` (`TclCompileStringTrimCmd`;
    /// subcommand-keyed).
    StringTrim => (29, "string trim"),
    /// `string trimleft string ?chars?` — `strtrimLeft`
    /// (`TclCompileStringTrimLCmd`; subcommand-keyed).
    StringTrimLeft => (30, "string trimleft"),
    /// `string trimright string ?chars?` — `strtrimRight`
    /// (`TclCompileStringTrimRCmd`; subcommand-keyed).
    StringTrimRight => (31, "string trimright"),
}
}

source_catalogue_enum! {
/// Typed identifier for an analyser per-command handler family.
///
/// The analyser's central dispatch
/// (`tcl_compiler::analyser` — `dispatch_command_handlers`) used to
/// chain ~28 `handle_*` methods, each re-checking the command-name
/// literal it owned.  The registry now stamps which handler family
/// owns a command form — a subcommand-level stamp (`namespace eval`
/// vs `namespace import`, `dict for` vs `dict update`) overrides the
/// command level, exactly like [`InlineCodegenHookId`] — and the
/// dispatcher performs one typed `match` on the resolved hook.  The
/// compiler keeps the handler implementations and their *shape*
/// guards (arity, braced-body, literalness); the registry keeps only
/// the catalogue.  A new variant gives the analyser a deliberate
/// compile-time match-exhaustion error until its arm is wired up.
///
/// Resolution matches the retired guards exactly: the head must be
/// the spec's own spelling (a `::`-qualified head resolves no hook,
/// as the literal guards never matched one) and a subcommand word
/// must match its `SubCommand` name exactly (no prefix abbreviation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnalyserHookId {
    /// `proc name params body` — records the proc and walks its body
    /// in a fresh proc scope.
    Proc => (0, "proc"),
    /// `tcl::OptProc name optlist body` — the Tcl `opt` package's
    /// automatic-option-parsing proc definer (`uplevel 1 [list ::proc
    /// $name args ...]`, so the real Tcl-level formal is the single
    /// literal word `args`; `optlist`'s own descriptor names are bound as
    /// locals in the body instead of being treated as a normal formal
    /// parameter list).  Records the proc the same last-definition-wins
    /// way a literal `proc` does — a real corpus idiom (`tk/library/
    /// safetk.tcl`) declares a throwaway 0-arg literal `proc` stub, then
    /// unconditionally redefines it this way.
    OptProc => (1, "argparse-style proc"),
    /// `apply {{params} body ?ns?} ?arg ...?` — models the lambda like
    /// a `proc` (params bind in a fresh scope; element 1 is the body).
    Apply => (2, "apply"),
    /// `uplevel #0 script` — the global-frame form opens an uplevel
    /// scope; every other `uplevel` shape falls through to the generic
    /// body recursion.
    Uplevel => (3, "uplevel"),
    /// `namespace eval ns ?body?` — opens a namespace child scope
    /// (stamped on `namespace`'s `eval` subcommand).
    NamespaceEval => (4, "namespace eval"),
    /// `namespace ensemble create ?options?` — records the enclosing
    /// namespace (and an explicit `-command` name) as an ensemble.
    NamespaceEnsemble => (5, "namespace ensemble"),
    /// `namespace import ?-force? pattern ...` — records literal
    /// import patterns; a dynamic pattern flips the dynamic-providers
    /// flag.
    NamespaceImport => (6, "namespace import"),
    /// `namespace export ?-clear? pattern ...` — records which command
    /// names the declaring namespace exposes; `-clear` resets the
    /// namespace's previously recorded patterns first. Gates whether a
    /// bareword call reached only through a wildcard `namespace import
    /// NS::*` may resolve to a command in `NS`: Tcl
    /// only imports names a source namespace actually exported
    /// (`Tcl_Export`, `tclNamesp.c`), so an unexported sibling command
    /// must stay unresolved through the import.
    NamespaceExport => (7, "namespace export"),
    /// `namespace forget ?pattern ...?` — records the removal of an
    /// imported alias. The counterpart of [`Self::NamespaceImport`]: an
    /// import edge has a lifecycle, and `namespace forget` ends it, so a
    /// bare call after the forget raises `invalid command name`
    /// (oracle tclsh 8.6.14 / 9.0.4). Recorded as an ordered event
    /// beside `namespace export`'s `-clear` tombstones.
    NamespaceForget => (8, "namespace forget"),
    /// `namespace path {ns ...}` — records the namespace's
    /// command-resolution search path.
    NamespacePath => (9, "namespace path"),
    /// `namespace unknown handler` — installing a handler makes
    /// command resolution unknowable (dynamic providers).
    NamespaceUnknown => (10, "namespace unknown"),
    /// `foreach varList list ?varList list ...? body` — defines the
    /// loop variables and walks the body.  Also stamped on the EDA
    /// `foreach_in_collection` (same shape).
    Foreach => (11, "foreach"),
    /// `switch ?options? string ?pattern body ...?` — walks each arm
    /// body and records `-regexp` patterns.
    Switch => (12, "switch"),
    /// `catch script ?resultVar? ?optionsVar?` — walks the guarded
    /// body and defines the result / options variables.
    Catch => (13, "catch"),
    /// `dict with dictVar body` — binds the keys of a
    /// constant-propagated dict value (stamped on `dict`'s `with`
    /// subcommand).
    DictWith => (14, "dict with"),
    /// `interp alias {} name {} target ?arg ...?` — records (or, in
    /// the four-word form, deletes) a current-interpreter alias
    /// (stamped on `interp`'s `alias` subcommand).
    InterpAlias => (15, "interp alias"),
    /// `interp eval path script` — evaluates the script in a *child*
    /// interpreter (a separate command/variable space).  The analyser opens
    /// an isolated scope so the child's `proc`/`var` definitions and calls do
    /// not merge into the parent namespace (stamped on `interp`'s `eval`
    /// subcommand).
    InterpEval => (16, "interp eval"),
    /// `interp create ?-safe? ?--? ?path?` — records the interpreter's
    /// existence and safe state in the analyser's interpreter-domain map
    /// (stamped on `interp`'s `create` subcommand).  A safe child's
    /// evaluation contexts hide every [`crate::Traits::SAFE_INTERP_HIDDEN`]
    /// command.
    InterpCreate => (17, "interp create"),
    /// `interp delete ?path ...?` — removes the recorded interpreter
    /// state (stamped on `interp`'s `delete` subcommand).
    InterpDelete => (18, "interp delete"),
    /// `interp hide path cmdName ?hiddenName?` — marks the command
    /// hidden in the target interpreter's domain (stamped on `interp`'s
    /// `hide` subcommand).
    InterpHide => (19, "interp hide"),
    /// `interp expose path hiddenName ?newName?` — re-exposes a hidden
    /// command in the target interpreter's domain (stamped on `interp`'s
    /// `expose` subcommand).
    InterpExpose => (20, "interp expose"),
    /// `rename oldName newName` — records a static rename / deletion;
    /// a dynamic operand reports back so the caller can widen the
    /// dynamic-providers flag.
    Rename => (21, "rename"),
    /// `oo::define class ?script | member args?` — extends a recorded
    /// class from the definition script or inline member form.
    OoDefine => (22, "oo::define"),
    /// `oo::objdefine object ...` — records the object variable so
    /// per-instance method extensions suppress unknown-method noise.
    OoObjdefine => (23, "oo::objdefine"),
    /// `package require ?-exact? name ?version?` — records the
    /// dependency (stamped on `package`'s `require` subcommand).
    PackageRequire => (24, "package require"),
    /// `package provide name ?version?` — records the provided
    /// package (stamped on `package`'s `provide` subcommand).
    PackageProvide => (25, "package provide"),
    /// `package ifneeded name version ?script?` — records that this
    /// document registers a *load script* for the named package
    /// (stamped on `package`'s `ifneeded` subcommand).
    ///
    /// The script body is arbitrary and runs later, in the global
    /// namespace, when some `package require` needs it — so its
    /// presence is what tells a package-derived load order that the
    /// mapping from require-site to the statements that actually run
    /// is *not* static.
    PackageIfneeded => (26, "package ifneeded"),
    /// `package prefer ?latest|stable?` — records the interpreter's
    /// version-selection mode change (stamped on `package`'s `prefer`
    /// subcommand), which decides whether a later `package require`
    /// takes the highest acceptable version or the highest acceptable
    /// *stable* one.
    PackagePrefer => (27, "package prefer"),
    /// `source fileName`, `source -encoding enc fileName`, or Tcl 9's
    /// `source -nopkg fileName` — records the source target.
    Source => (28, "source"),
    /// `regexp` / `regsub` — records literal / constant-propagated
    /// pattern arguments for highlighting.
    RegexPatternCapture => (29, "regexp capture binding"),
    /// `load libFile ?prefix? ?interp?` — brings a shared library's
    /// commands in at runtime: flips the dynamic-providers flag.
    Load => (30, "load"),
}
}

/// Compile-time constant folder.
///
/// Given resolved constant argument strings, returns the computed
/// result string or `None` if the fold cannot be performed (e.g.
/// arguments are not constant, or the operation is not supported).
pub type ConstFoldFn = fn(args: &[&str]) -> Option<String>;

// `TclVersion` moved to the foundational `tcl-dialect` crate
// (dialect-profile-model.md §3) so behaviour consumers below the registry
// share it; re-exported here for backwards compatibility and the prelude.
pub use tcl_dialect::TclVersion;

/// A Tcl-version-aware constant folder.  `version` is `None` when the target
/// dialect doesn't name a specific Tcl release, in which case the fold must
/// return only the dialect-invariant result every version agrees on (or
/// `None`).  Used for commands whose compile-time value depends on the Tcl
/// version (`string is`, `format`, `scan`).
pub type VersionedConstFoldFn = fn(args: &[&str], version: Option<TclVersion>) -> Option<String>;

source_catalogue_enum! {
/// Typed identifier for a command whose return type depends on the call.
///
/// [`crate::CommandSpec::return_type`] is one fact per command, which is right
/// only while the result shape holds still. Several core commands hand back a
/// different *kind* of value depending on how they were called: `regexp`
/// counts matches but `regexp -inline` returns the matched substrings
/// instead, `regsub` returns a replacement count until its `varName` is
/// omitted and it returns the substituted string instead. Typing every call by the
/// command's usual result makes the compiler confidently wrong about the
/// others: iterating a `regexp -all -inline` result once drew a
/// shimmer warning saying the list "has int intrep".
///
/// The spec names the algorithm; [`crate::return_type`] keeps it. That split
/// is what lets the rule be a real program — `lsearch` has to know that
/// `-inline` dominates `-subindices`, which no table of switch/type pairs
/// expresses — while the spec stays declarative data a `.tclspec` pack can
/// author by name. A new variant gives the dispatcher a deliberate
/// match-exhaustion error until its arm is written.
///
/// An algorithm names a type only where the intrep is *guaranteed* and
/// answers "unknown" otherwise, so several forms below type as unknown even
/// though their documented result is a list — [`crate::return_type`] has the
/// three reasons and the tclsh evidence for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReturnTypeHookId {
    /// `regexp ?switches? exp string ?matchVar ...?` — `-about` skips matching
    /// altogether and returns a guaranteed two-element
    /// `{subexpressionCount propertyList}`. `-inline` returns the matched
    /// substrings, which is a list only once something matches, so it types as
    /// unknown rather than as the flag. Otherwise the 0/1 match flag, or the
    /// match count under `-all`.
    Regexp => (0, "regexp -inline / -about"),
    /// `lsearch ?options? list pattern` — `-all` and `-inline` both type as
    /// unknown, for different reasons: `-all` builds a list only once it
    /// matches, and a bare `-inline` returns one element straight out of the
    /// source list, so its intrep is the caller's. `-subindices` turns a plain
    /// index into the full index path, which *is* a guaranteed list (the
    /// no-match answer is `-1 0`); otherwise an index.
    Lsearch => (1, "lsearch -all / -inline / -subindices"),
    /// `regsub ?switches? exp string subSpec ?varName?` — the substituted
    /// string when `varName` is omitted, the replacement count when it is not.
    Regsub => (2, "regsub with or without varName"),
    /// `scan string format ?varName ...?` — the variable-writing form returns
    /// the conversion count. The inline (no-`varName`) form yields the
    /// converted values, a list only once something converts, so it types as
    /// unknown.
    Scan => (3, "scan inline or variable-writing"),
    /// `pid ?fileId?` — bare `pid` is this process's id. The `fileId` form
    /// yields the pipeline's process ids, but answers a pure string on a
    /// channel that is not a pipeline, so it types as unknown.
    Pid => (4, "pid with or without fileId"),
}
}

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
