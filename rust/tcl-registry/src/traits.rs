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

//! Behavioural trait flags for commands.
//!
//! Each bit replaces one `bool` field from `CommandSpec`.
//! Consumers query traits via `spec.traits.contains(Traits::CONTROL_FLOW)`
//! instead of matching on command name strings.

use bitflags::bitflags;

bitflags! {
    /// Declarative behavioural traits for a command.
    ///
    /// Packed into a single `u64` — compact storage, fast intersection
    /// and containment queries on a single spec. Whole-registry
    /// trait-membership queries
    /// ([`crate::registry::CommandRegistry::commands_with_trait`])
    /// scan the spec table in O(N) today; a precomputed trait index
    /// is a future optimisation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Traits: u64 {
        // Control flow
        /// Command is a control-flow construct (`if`, `for`, `while`, `switch`).
        const CONTROL_FLOW              = 1 << 0;
        /// Language keyword for semantic token classification.
        const LANGUAGE_KEYWORD          = 1 << 1;
        /// First expression argument is in boolean context (`if`, `while`, `for`).
        const HAS_BOOLEAN_COND          = 1 << 2;
        /// Unconditionally terminates the current block (`error`, `return`, `exit`).
        const TERMINATES_BLOCK          = 1 << 3;

        // Loop/body structure
        /// Contains a loop body (`for`, `while`, `foreach`).
        const HAS_LOOP_BODY             = 1 << 4;
        /// Forbid inlining body arguments.
        const NEVER_INLINE_BODY         = 1 << 5;
        /// CFG header with list-expression args evaluated once (`foreach`, `lmap`).
        const LOOP_LIST_HEADER          = 1 << 6;

        // Purity and optimisation
        /// Side-effect-free command.
        const PURE                      = 1 << 7;
        /// Candidate for common subexpression elimination.
        const CSE_CANDIDATE             = 1 << 8;
        /// Pure evaluation (`expr` — side-effect-free when braced).
        const PURE_EVALUATION           = 1 << 9;

        // Variable semantics
        /// Defines a procedure (`proc`).
        const DEFINES_PROCEDURE         = 1 << 10;
        /// Destroys/removes a variable (`unset`).
        const DESTROYS_VARIABLE         = 1 << 11;
        /// Reads the target variable before writing (`incr`, `append`, `lappend`).
        const READS_BEFORE_WRITE        = 1 << 12;
        /// Creates a scope alias — upvar-like binding (`upvar`, `global`, `variable`).
        const CREATES_SCOPE_ALIAS       = 1 << 13;
        /// Creates a runtime control-flow barrier (`eval`, `uplevel`, `upvar`).
        const CREATES_BARRIER           = 1 << 14;

        // Analysis check dispatch
        /// Evaluates code dynamically (`eval`, `uplevel`).
        const EVALUATES_CODE            = 1 << 15;
        /// Performs backslash/variable substitution (`subst`).
        const PERFORMS_SUBSTITUTION      = 1 << 16;
        /// Opens a channel (`open`).
        const OPENS_CHANNEL             = 1 << 17;
        /// Sources a file (`source`).
        const SOURCES_FILE              = 1 << 18;
        /// Has a switch body (`switch`).
        const HAS_SWITCH_BODY           = 1 << 19;
        /// String/list confusion risk (`append`).
        const STRING_LIST_CONFUSION     = 1 << 20;
        /// Configures a channel (`fconfigure`, `chan configure`).
        const CONFIGURES_CHANNEL        = 1 << 21;
        /// Has `interp eval` subcommand.
        const HAS_INTERP_EVAL           = 1 << 22;
        /// Has destructive operations (`file delete`, `namespace delete`).
        const HAS_DESTRUCTIVE_OPS       = 1 << 23;
        /// iRules event handler (`when`).
        const IS_EVENT_HANDLER          = 1 << 24;
        /// Returns unnormalised HTTP path/URI/query.
        const UNNORMALISED_HTTP_GETTER  = 1 << 25;

        // Output/value traits
        /// Returns a filesystem path (`pwd`, `file join`).
        const RETURNS_PATH              = 1 << 26;
        /// Performs unescaping/decoding (`subst`, `URI::decode`).
        const IS_UNESCAPE              = 1 << 27;
        /// Produces a canonical Tcl list (`list`, `concat`).
        const PRODUCES_CANONICAL_LIST   = 1 << 28;

        /// Quotes its arguments into a well-formed command reference: when
        /// the first argument is a literal command name, evaluating the
        /// result invokes that command with the remaining arguments
        /// appended verbatim (`list cmd $a $b` → `cmd $a $b`, each word
        /// preserved regardless of its runtime value). Set on `list` only —
        /// **not** `concat`, whose plain string-join does not give the same
        /// per-word quoting guarantee for a dynamic value, so it is unsafe
        /// to reinterpret the same way. The idiomatic way to build a
        /// deferred callback / command prefix around a dynamic value
        /// (`package ifneeded name ver [list apply {params} {body} $dir]`,
        /// `button .b -command [list doSomething $x]`) rather than a
        /// literal braced prefix. Consulted wherever a
        /// [`crate::arg_role::ArgRole::CommandPrefix`] /
        /// [`crate::arg_role::ArgRole::Body`] /
        /// [`crate::arg_role::ArgRole::LambdaLiteral`] argument position
        /// needs to recognise this quoting shape generically — no command
        /// name appears in the consumer.
        const BUILDS_COMMAND_PREFIX     = 1 << 30;

        // Safety
        /// Inherently dangerous command.
        const UNSAFE                    = 1 << 29;
        /// Password option command.
        const PASSWORD_OPTION           = 1 << 31;

        // iRules-specific
        /// Side-switching command (`clientside`/`serverside`).
        const IS_SIDE_SWITCH            = 1 << 32;
        /// Must appear at iRules top level (`proc`, `when`, `timing`).
        const IRULES_TOP_LEVEL_ONLY     = 1 << 33;
        /// TclOO metaclass (`oo::class`, `oo::abstract`).
        const IS_OO_METACLASS           = 1 << 34;

        // Codegen/diagram
        /// Included in diagram extraction.
        const DIAGRAM_ACTION            = 1 << 35;
        /// Needs `startCommand` bytecode instruction.
        const NEEDS_START_CMD           = 1 << 36;

        // Taint
        /// Command is a taint sink (absorbs tainted data).
        const TAINT_SINK                = 1 << 37;
        /// Command returns attacker-controlled data
        /// (`gets`, `read`, `exec`, `socket`, …).
        const TAINT_SOURCE              = 1 << 38;
        /// Command operates on attacker-controlled iRules data
        /// (any reachable form of `HTTP::*` / `URI::*` / `IP::*` /
        /// `TCP::*` / `UDP::*` / `SSL::*` / `STREAM::*`).
        const IRULES_DATA_GETTER        = 1 << 39;

        /// Creates a runtime scope-alias barrier whose VarWrite
        /// args are vararg lists (`global x y z`, `variable a b c`,
        /// `upvar 1 a b 1 c d`).  The analyser's `var_scoping` pass
        /// handles the per-arg list; SSA must not produce partial
        /// defs from `arg_roles[0]`.
        const CREATES_DYNAMIC_BARRIER   = 1 << 40;

        /// Command invokes a user-defined Tcl procedure named by
        /// its first argument.  Set on the iRules `call` command
        /// (`call PROC_NAME ?ARGS?`).  Used by the LSP completion
        /// provider to surface user-proc names — and only those,
        /// not built-in commands — at word-index 1.
        const INVOKES_USER_PROC         = 1 << 41;

        /// Core Tcl built-in that the bytecode compiler special-cases
        /// (or that is otherwise load-bearing as a literal command
        /// word).  Re-invoking such a command through a `$var`
        /// command alias would defeat byte-compilation or change
        /// semantics, so the minifier never rewrites these heads to
        /// `$alias`.  Single source of truth for the minifier's
        /// former `_BUILTIN_SKIP` list; query via
        /// [`crate::registry::CommandRegistry::is_byte_compiled`].
        const BYTE_COMPILED             = 1 << 42;

        /// Command head incidentally matches the
        /// `HEAD NAME BRACED BRACED` four-token shape but is **not** a
        /// proc-factory wrapper, so the signature scanner must not
        /// treat it as one.  Single source of truth for the analyser's
        /// former `_FACTORY_SKIP_HEADS` list (registered heads only;
        /// non-command heads like `method` / `itcl::class` are handled
        /// by a small residual set in the scanner).
        const NOT_PROC_FACTORY          = 1 << 43;

        /// Command whose codegen always lowers to a dedicated runtime
        /// helper (or to a structured IR node) and never falls back to
        /// the interpreter — so a call to it needs no runtime frame in
        /// the callee.  The var-escape analysis treats only these as
        /// frame-free.  Single source of truth for the former
        /// `_FRAMELESS_RUNTIME_COMMANDS` allow-list.  Keep audited:
        /// stamping a command that secretly eval-falls-back would
        /// break eval-inside-proc semantics in escape-free procs.
        const FRAMELESS_RUNTIME         = 1 << 44;

        /// First argument is a variable *name* (read / write / modify),
        /// not a value — `set` / `incr` / `append` / `lappend` / `unset`.
        /// The var-escape analysis uses this to detect dynamic-name forms
        /// (`set $n value`). Single source of truth for the former
        /// `NAME_FIRST_COMMANDS` allow-list.
        const FIRST_ARG_VARNAME         = 1 << 45;

        /// A `VarRead`-role argument names the *whole* array, not a single
        /// element (`array` / `parray`), so a write to any element is
        /// observed by the read. Single source of truth for the former
        /// `WHOLE_ARRAY_COMMANDS` allow-list.
        const WHOLE_ARRAY_ARG           = 1 << 46;

        /// Executes a body through the interpreter, opening a
        /// name-resolution channel back into the local frame — `eval` /
        /// `uplevel` / `apply` / `source` / `namespace` / `interp`. The
        /// var-escape slot resolver treats a frame reached this way as
        /// hash-backed. Single source of truth for the former
        /// `DYNAMIC_EVAL_COMMANDS` allow-list. (Distinct from
        /// `HAS_INTERP_EVAL` / `CREATES_DYNAMIC_BARRIER`, which only some
        /// of these carry.)
        const DYNAMIC_EVAL_BODY         = 1 << 47;

        /// (Subcommand) introspects program state by variable *name* —
        /// `info exists|vars|locals|args|default`. The var-escape slot
        /// resolver treats the named variable as ineligible for a slot.
        /// Single source of truth for the former
        /// `INFO_INTROSPECTING_SUBCMDS` list.
        const INTROSPECTS_BY_NAME       = 1 << 48;

        /// (Subcommand) targets a variable by *name* —
        /// `trace add|remove|info|variable|vdelete|vinfo`. Used by the
        /// var-escape slot resolver. Single source of truth for the former
        /// `TRACE_NAME_TARGETING` list.
        const TARGETS_VARIABLE_BY_NAME  = 1 << 49;

        /// Aliases / reads / writes variables through the frame's *hash
        /// bucket* by name (`upvar` / `global` / `variable` / `lassign` /
        /// `lset` / `regexp` / `regsub` / `scan` / `binary` / `vwait` /
        /// `tkwait`), so a named variable it touches cannot live in an
        /// indexed slot. Single source of truth for the former
        /// `FRAME_HASH_BUILTINS` list.
        const FRAME_HASH_BUILTIN        = 1 << 50;

        /// A Tcl auto-loading / library proc that user code is expected to
        /// redefine (`unknown`, `auto_*`, `pkg_*`, `tclLog`,
        /// `tcl_findLibrary`, the `tcl_*Word*` helpers, …), so redefining
        /// it must not fire the W113 "overrides a built-in" warning.
        /// Single source of truth for the former
        /// `OVERRIDABLE_LIBRARY_PROCS` list.
        const OVERRIDABLE_LIBRARY_PROC  = 1 << 51;

        /// The WASM backend lowers this command to a structural construct
        /// that imports / emits no runtime helper of its own (`foreach`,
        /// `namespace`, `package`, `proc`).  Consulted by the WASM import
        /// collector.
        const WASM_EMITS_NOTHING        = 1 << 52;

        /// The registry's simple `min..=max` [`crate::Arity`] is a coarse
        /// floor/ceiling only — this command's real grammar is a clause
        /// chain validated by [`crate::spec::CommandSpec::clause_shape_check`]
        /// (`if`'s `elseif`/`else` chain). The generic arity floor/ceiling
        /// diagnostic (E002/E003) skips a command carrying this trait so its
        /// dedicated structural diagnostic owns arity together with clause
        /// shape — one precise diagnostic per malformed call instead of a
        /// redundant generic one alongside it.
        const STRUCTURALLY_CHECKED_ARITY = 1 << 54;

        /// The command concatenates its *entire* argument list into a single
        /// expression (`expr` — `expr $a + $b` evaluates the one expression
        /// `$a + $b`). This differs from commands whose expression is a single
        /// bounded argument (`if` / `while` / `for` conditions,
        /// `control::assert`'s first arg), which mark only that arg
        /// [`crate::arg_role::ArgRole::Expr`]. Consulted by the formatter's
        /// `enforceBracedExpr` pass to decide whether to brace the whole tail
        /// (`expr {$a + $b}`) or just the marked argument. Kept separate from
        /// the `Expr` arg-role so widening it does not perturb the analyser
        /// passes (expr re-lexing, W110) that consume the role.
        const EXPR_CONCATENATES_ARGS    = 1 << 53;

        /// (Subcommand) installs or removes an active variable trace on a
        /// named target — `trace add|remove|variable|vdelete` (not
        /// `info`/`vinfo`, which only *query* trace state). A variable
        /// carrying an active trace can run arbitrary handler code on
        /// read/write/unset, and a write handler can rewrite the value
        /// being stored — so no compiler pass may treat a read of this
        /// variable as equivalent to its last literal assignment, or elide
        /// an assignment to it as dead. Single source of truth for the
        /// module-wide traced-variable fact consumed by the propagation
        /// optimiser (`O102` load-forwarding) and dead-store elimination.
        const ESTABLISHES_VARIABLE_TRACE = 1 << 56;

        /// Transfers control relative to the enclosing loop, frame, or
        /// coroutine instead of falling through to the next statement —
        /// `break` / `continue` (loop exit / re-entry), `tailcall` (frame
        /// replacement), `yield` / `yieldto` (coroutine suspension).
        /// Deliberately distinct from [`Traits::TERMINATES_BLOCK`], which
        /// marks commands that *unwind* the enclosing block/proc
        /// (`return` / `error` / `exit` / `throw`) — a `break` leaves only
        /// the loop and a `yield` resumes in place, so stamping them
        /// `TERMINATES_BLOCK` would corrupt the dead-end analyses (W241,
        /// taint guard propagation) that consume that trait.  Together the
        /// two traits give consumers the full "diverts control flow" set;
        /// the inline-proc code action and the inliner's splice-safety
        /// query are the current consumers.
        const TRANSFERS_CONTROL          = 1 << 61;

        /// Teardown/removal command (or subcommand) for which a bare
        /// `catch {…}` with no result variable is the documented
        /// fire-and-forget idiom: the operation errors when its target is
        /// already gone, and ignoring that failure is intentional —
        /// `close` / `unset` / `rename foo {}`, `after cancel`,
        /// `chan close`, `array unset`, `dict unset`, `interp delete`,
        /// `file delete`, `namespace delete|forget`.  Single source of
        /// truth for W302's suppression set.  Narrower than
        /// [`crate::spec::SubCommand::destructive`] (`file rename` /
        /// `file mkdir` are destructive but their failures are real
        /// errors, not expected teardown noise).
        const FIRE_AND_FORGET_TEARDOWN   = 1 << 62;

        /// Command is a math-operator head (`+`, `eq`, `ne`, `in`, `ni`,
        /// and the `tcl::mathop::*` spellings): a real callable command in
        /// every dialect whose profile has `operators_as_commands`, but
        /// *never* a command head where operators live only inside `expr`
        /// (F5 iRules; `tk` when modelled). Replaces the retired
        /// `NON_IRULES_OPERATORS` membership tag as the operator-head
        /// marker (dialect-profile-model.md §9, Milestone 5).
        const OPERATOR_COMMAND           = 1 << 63;

        /// `TclOO` `next` / `nextto` — invokes the next implementation of the
        /// *currently executing* method along the receiver's MRO. Its
        /// callee's arity is resolvable only from the enclosing method's
        /// call-site context (which class/method body the call textually
        /// sits in), never from a fixed registry range, so both commands
        /// keep [`crate::Arity::any`] / [`crate::Arity::at_least`] and the
        /// analyser queues a context-aware candidate for any command
        /// carrying this trait instead
        /// (`Analyser::queue_next_arity_candidate`). Set on `next` and
        /// `nextto`; `nextto`'s explicit target-class first word is
        /// distinguished structurally via an [`crate::arg_role::ArgRole::Name`]
        /// at argument index 0, not by command name.
        const TCLOO_NEXT_CHAIN           = 1 << 55;

        /// Raises a *catchable* exception — completes `TCL_ERROR`, which
        /// `catch` / `try` intercept: `error` (`Tcl_ErrorObjCmd`,
        /// `generic/tclCmdAH.c`) and `throw` (`TclNRThrowObjCmd`,
        /// `generic/tclBasic.c`, 8.6+). The strict subset of
        /// [`Self::TERMINATES_BLOCK`] that sources an enclosing `try`'s
        /// on-error edge: `exit` (`Tcl_ExitObjCmd`) terminates the
        /// process rather than unwinding through exception ranges, and
        /// `return` pops the frame with `TCL_RETURN`, so neither is a
        /// throw point. Consumed by the CFG builder's throw-block
        /// recording.
        const CATCHABLE_THROW           = 1 << 57;

        /// Jumps to the innermost enclosing loop's *post-loop* target —
        /// `break`, which completes `TCL_BREAK` (`Tcl_BreakObjCmd`,
        /// `generic/tclCmdAH.c`); inside a compiled loop the bytecode
        /// compiler rewrites it into a jump to the loop's break fixup
        /// site (`TclCompileBreakCmd`, `generic/tclCompCmds.c`).
        /// Loop-jump classification for the CFG builder's
        /// `break`/`continue` edge lowering and the inline emitters'
        /// straight-line-body gate; paired with
        /// [`Self::CONTINUES_LOOP`].
        const BREAKS_LOOP               = 1 << 58;

        /// Jumps to the innermost enclosing loop's *next-iteration*
        /// target — `continue`, which completes `TCL_CONTINUE`
        /// (`Tcl_ContinueObjCmd`, `generic/tclCmdAH.c`;
        /// `TclCompileContinueCmd`, `generic/tclCompCmds.c`). The
        /// other half of the loop-jump classification — see
        /// [`Self::BREAKS_LOOP`].
        const CONTINUES_LOOP            = 1 << 59;

        /// Replaces the current procedure's frame — `tailcall`
        /// (`TclNRTailcallObjCmd`, `generic/tclBasic.c`, 8.6+), which
        /// schedules its command to run after the frame pops and always
        /// completes `TCL_RETURN`, so control never resumes in the
        /// calling body. Deliberately *not* [`Self::TERMINATES_BLOCK`]:
        /// the analysis CFG promotes it to a proc-exit terminator, but
        /// codegen must keep the plain fall-through call shape so the
        /// emitted bytecode matches C Tcl's.
        const REPLACES_FRAME            = 1 << 60;

        /// Hidden when an interpreter is made **safe** — the command is
        /// *not* part of C Tcl's safe-interpreter command set (its
        /// `CmdInfo` row lacks `CMD_IS_SAFE`, or it is a whole-command
        /// row of `unsafeEnsembleCommands` — `tclBasic.c`, 9.0.4:
        /// `cd`, `encoding`, `exec`, `exit`, `fconfigure`, `file`,
        /// `glob`, `load`, `open`, `pwd`, `socket`, `source`,
        /// `unload`).  A call inside a safe interpreter's evaluation
        /// context errors `invalid command name` unless the command was
        /// re-exposed (`interp expose`) or reached via
        /// `interp invokehidden`; the analyser's safe-context walk
        /// consults this flag generically — no command name appears in
        /// the consumer (issue #945 fault 7).
        const SAFE_INTERP_HIDDEN        = 1 << 61;
    }
}

/// Clause/block words that behave as [`Traits::LANGUAGE_KEYWORD`] tokens but
/// have no standalone `CommandSpec` — they are never independently invocable
/// (`else` only means anything as an `if` clause), so they cannot carry a
/// registry entry of their own the way `if`/`foreach`/`proc` do.
///
/// Single source of truth for every consumer that needs "the real Tcl
/// keywords a `CommandSpec`-driven scan alone would miss": the LSP's
/// semantic-token classifier
/// (`tcl_lsp_core::semantic_tokens::LANGUAGE_KEYWORD_SUB_KEYWORDS`, which
/// unions in its own further residue — the `TclOO` method-body helpers
/// `callback`/`mymethod`/`link`, which are context-sensitive rather than
/// unconditional keywords) and the static TextMate-grammar generator
/// (`xtask`'s `gen_tmlanguage_keywords`, which unions in the iRules-only
/// `when`). Keeping this list here instead of duplicating it in both means a
/// new clause word is added once and both consumers pick it up.
pub const CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC: &[&str] =
    &["else", "elseif", "on", "trap", "finally"];

/// Clause *noise* words: accepted by a clause grammar as optional filler but
/// deliberately **not** highlighted as keywords — today only `if`'s optional
/// `then` (`if {c} then {b}`), which `if`'s arg-role resolver and clause-shape
/// checker match by literal value.
///
/// Kept separate from [`CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC`] because the two
/// lists serve different consumers: the keyword list drives highlighting (the
/// semantic-token classifier and the TextMate-grammar generator, which must
/// not paint `then`), while the union of both lists is "every word a clause
/// grammar matches by value", which value-sensitive rewriters (the minifier's
/// argument aliasing — rewriting a literal `then` to `$alias` would break
/// `if`'s clause parsing) must keep literal.
pub const CLAUSE_NOISE_KEYWORDS: &[&str] = &["then"];
