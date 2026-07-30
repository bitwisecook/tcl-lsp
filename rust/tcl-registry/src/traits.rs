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
//! Each flag replaces one `bool` field from `CommandSpec`. Consumers query
//! traits via `spec.traits.contains(Traits::CONTROL_FLOW)` instead of matching
//! on command name strings.
//!
//! # Why an enum and not `bitflags`
//!
//! Every flag is declared **once**, by name, in `declare_traits!` below. Its
//! bit is the enum discriminant the compiler assigns — no bit number is ever
//! written down, so two flags cannot be given the same one. That is not a
//! checked invariant; it is an unrepresentable state.
//!
//! This matters because it already went wrong. Under `bitflags` these were
//! spelled `1 << 61` and `1 << 61`:
//!
//! ```text
//! const TRANSFERS_CONTROL  = 1 << 61;
//! const SAFE_INTERP_HIDDEN = 1 << 61;
//! ```
//!
//! `bitflags` cannot reject that — two constants sharing a value is a
//! supported feature there, used for composite masks like `const RW = R | W`.
//! So the `u64` silently held 65 flags in 64 bits and the two names became one
//! flag: every `SAFE_INTERP_HIDDEN` command (`file`, `source`, `encoding`,
//! `open`, …) read as control-transferring and therefore as *frame-sensitive*,
//! suppressing the inline-proc code action on all of them, while `break`,
//! `continue`, `yield`, `yieldto` and `tailcall` read as safe-interp-hidden.
//!
//! The macro also generates [`Trait::name`] as an exhaustive `match`, so a new
//! flag fails to compile until it is named — the string table cannot drift
//! from the flags either.

use std::fmt;
use std::ops::{BitOr, BitOrAssign};

/// Declare every behavioural trait exactly once.
///
/// `Variant => CONST_NAME;` generates the [`Trait`] variant, the [`Traits`]
/// single-flag constant, and the arm of [`Trait::name`]. Bits come from the
/// discriminants the compiler assigns, so ordering is free and duplication is
/// impossible.
///
/// Each doc comment is copied onto *both* generated items, so `Self` in an
/// intra-doc link would resolve to `Trait` on the variant and `Traits` on the
/// constant. Spell such links against `Traits` explicitly, never `Self`.
macro_rules! declare_traits {
    ($( $(#[$meta:meta])* $variant:ident => $konst:ident );* $(;)?) => {
        /// The identity of a single behavioural trait.
        ///
        /// One variant per flag. Its bit is its discriminant — see the module
        /// docs for why no bit is written by hand.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        #[allow(missing_docs)] // Documented on the matching `Traits` constant.
        pub enum Trait {
            $( $(#[$meta])* $variant ),*
        }

        impl Trait {
            /// Every declared trait, in declaration order.
            pub const ALL: &'static [Trait] = &[ $( Trait::$variant ),* ];

            /// The trait's screaming-snake name, as written in a spec literal.
            ///
            /// Exhaustive by construction: adding a variant without a name
            /// here is a compile error.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self { $( Trait::$variant => stringify!($konst) ),* }
            }
        }

        impl Traits {
            $( $(#[$meta])* pub const $konst: Self = Self::of(&[Trait::$variant]); )*
        }
    };
}

/// A set of behavioural traits.
///
/// A `u128` bitset over [`Trait`]. The only way to set a bit is to name a
/// `Trait`, so the set cannot contain a flag that was never declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Traits(u128);

impl Trait {
    /// This trait's bit. The discriminant *is* the bit index.
    const fn bit(self) -> u128 {
        1u128 << (self as u8)
    }
}

impl Traits {
    /// The empty set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Build a set from a slice of traits, usable in `const` context.
    #[must_use]
    pub const fn of(list: &[Trait]) -> Self {
        let (mut acc, mut i) = (0u128, 0);
        while i < list.len() {
            acc |= list[i].bit();
            i += 1;
        }
        Self(acc)
    }

    /// Whether no trait is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every trait in `other` is set here.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether any trait is set in both.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Set union, usable in `const` context (`BitOr` cannot be).
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Set intersection — the traits set in both.
    ///
    /// The counterpart to [`Self::union`], for a consumer that wants to
    /// *narrow* a set to a mask rather than test it
    /// ([`Self::contains`] / [`Self::intersects`] answer a question; this
    /// returns the overlap). Used by
    /// [`crate::registry::CommandRegistry::unit_linkage`] to report only the
    /// [`crate::UNIT_LINKAGE_TRAITS`] a command carries.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Add every trait in `other`.
    pub const fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Remove every trait in `other`.
    pub const fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// How many traits are set.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// The traits set here, in declaration order.
    ///
    /// Replaces `bitflags`' `iter_names`, and is the single source for every
    /// consumer that needs to render a trait set as names.
    pub fn iter(self) -> impl Iterator<Item = Trait> {
        Trait::ALL
            .iter()
            .copied()
            .filter(move |t| self.0 & t.bit() != 0)
    }

    /// The names of the traits set here, in declaration order.
    pub fn iter_names(self) -> impl Iterator<Item = &'static str> {
        self.iter().map(Trait::name)
    }
}

impl BitOr for Traits {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Traits {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl FromIterator<Trait> for Traits {
    fn from_iter<I: IntoIterator<Item = Trait>>(iter: I) -> Self {
        let mut out = Self::empty();
        for t in iter {
            out.0 |= t.bit();
        }
        out
    }
}

impl fmt::Display for Traits {
    /// `A | B | C`, or `(empty)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("(empty)");
        }
        let mut first = true;
        for name in self.iter_names() {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str(name)?;
            first = false;
        }
        Ok(())
    }
}

declare_traits! {
    // Control flow
    /// Command is a control-flow construct (`if`, `for`, `while`, `switch`).
    ControlFlow => CONTROL_FLOW;
    /// Language keyword for semantic token classification.
    LanguageKeyword => LANGUAGE_KEYWORD;
    /// First expression argument is in boolean context (`if`, `while`, `for`).
    HasBooleanCond => HAS_BOOLEAN_COND;
    /// Unconditionally terminates the current block (`error`, `return`, `exit`).
    TerminatesBlock => TERMINATES_BLOCK;

    // Loop/body structure
    /// Contains a loop body (`for`, `while`, `foreach`).
    HasLoopBody => HAS_LOOP_BODY;
    /// Forbid inlining body arguments.
    NeverInlineBody => NEVER_INLINE_BODY;
    /// CFG header with list-expression args evaluated once (`foreach`, `lmap`).
    LoopListHeader => LOOP_LIST_HEADER;

    // Purity and optimisation
    /// Side-effect-free command.
    Pure => PURE;
    /// Candidate for common subexpression elimination.
    CseCandidate => CSE_CANDIDATE;
    /// Pure evaluation (`expr` — side-effect-free when braced).
    PureEvaluation => PURE_EVALUATION;

    // Variable semantics
    /// Defines a procedure (`proc`).
    DefinesProcedure => DEFINES_PROCEDURE;
    /// Destroys/removes a variable (`unset`).
    DestroysVariable => DESTROYS_VARIABLE;
    /// Reads the target variable before writing (`incr`, `append`, `lappend`).
    ReadsBeforeWrite => READS_BEFORE_WRITE;
    /// Creates a scope alias — upvar-like binding (`upvar`, `global`, `variable`).
    CreatesScopeAlias => CREATES_SCOPE_ALIAS;
    /// Creates a runtime control-flow barrier (`eval`, `uplevel`, `upvar`).
    CreatesBarrier => CREATES_BARRIER;

    // Analysis check dispatch
    /// Evaluates code dynamically (`eval`, `uplevel`).
    EvaluatesCode => EVALUATES_CODE;
    /// Performs backslash/variable substitution (`subst`).
    PerformsSubstitution => PERFORMS_SUBSTITUTION;
    /// Opens a channel (`open`).
    OpensChannel => OPENS_CHANNEL;
    /// Sources a file (`source`).
    SourcesFile => SOURCES_FILE;
    /// Has a switch body (`switch`).
    HasSwitchBody => HAS_SWITCH_BODY;
    /// String/list confusion risk (`append`).
    StringListConfusion => STRING_LIST_CONFUSION;
    /// Configures a channel (`fconfigure`, `chan configure`).
    ConfiguresChannel => CONFIGURES_CHANNEL;
    /// Has `interp eval` subcommand.
    HasInterpEval => HAS_INTERP_EVAL;
    /// Has destructive operations (`file delete`, `namespace delete`).
    HasDestructiveOps => HAS_DESTRUCTIVE_OPS;
    /// iRules event handler (`when`).
    IsEventHandler => IS_EVENT_HANDLER;
    /// Returns unnormalised HTTP path/URI/query.
    UnnormalisedHttpGetter => UNNORMALISED_HTTP_GETTER;

    // Output/value traits
    /// Returns a filesystem path (`pwd`, `file join`).
    ReturnsPath => RETURNS_PATH;
    /// Performs unescaping/decoding (`subst`, `URI::decode`).
    IsUnescape => IS_UNESCAPE;
    /// Produces a canonical Tcl list (`list`, `concat`).
    ProducesCanonicalList => PRODUCES_CANONICAL_LIST;

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
    BuildsCommandPrefix => BUILDS_COMMAND_PREFIX;

    // Safety
    /// Inherently dangerous command.
    Unsafe => UNSAFE;
    /// Password option command.
    PasswordOption => PASSWORD_OPTION;

    // iRules-specific
    /// Side-switching command (`clientside`/`serverside`).
    IsSideSwitch => IS_SIDE_SWITCH;
    /// Must appear at iRules top level (`proc`, `when`, `timing`).
    IrulesTopLevelOnly => IRULES_TOP_LEVEL_ONLY;
    /// `TclOO` metaclass (`oo::class`, `oo::abstract`).
    IsOoMetaclass => IS_OO_METACLASS;

    // Codegen/diagram
    /// Included in diagram extraction.
    DiagramAction => DIAGRAM_ACTION;
    /// Needs `startCommand` bytecode instruction.
    NeedsStartCmd => NEEDS_START_CMD;

    // Taint
    /// Command is a taint sink (absorbs tainted data).
    TaintSink => TAINT_SINK;
    /// Command returns attacker-controlled data
    /// (`gets`, `read`, `exec`, `socket`, …).
    TaintSource => TAINT_SOURCE;
    /// Command operates on attacker-controlled iRules data
    /// (any reachable form of `HTTP::*` / `URI::*` / `IP::*` /
    /// `TCP::*` / `UDP::*` / `SSL::*` / `STREAM::*`).
    IrulesDataGetter => IRULES_DATA_GETTER;

    /// Creates a runtime scope-alias barrier whose `VarWrite`
    /// args are vararg lists (`global x y z`, `variable a b c`,
    /// `upvar 1 a b 1 c d`).  The analyser's `var_scoping` pass
    /// handles the per-arg list; SSA must not produce partial
    /// defs from `arg_roles[0]`.
    CreatesDynamicBarrier => CREATES_DYNAMIC_BARRIER;

    /// Command invokes a user-defined Tcl procedure named by
    /// its first argument.  Set on the iRules `call` command
    /// (`call PROC_NAME ?ARGS?`).  Used by the LSP completion
    /// provider to surface user-proc names — and only those,
    /// not built-in commands — at word-index 1.
    InvokesUserProc => INVOKES_USER_PROC;

    /// Core Tcl built-in that the bytecode compiler special-cases
    /// (or that is otherwise load-bearing as a literal command
    /// word).  Re-invoking such a command through a `$var`
    /// command alias would defeat byte-compilation or change
    /// semantics, so the minifier never rewrites these heads to
    /// `$alias`.  Single source of truth for the minifier's
    /// former `_BUILTIN_SKIP` list; query via
    /// [`crate::registry::CommandRegistry::is_byte_compiled`].
    ByteCompiled => BYTE_COMPILED;

    /// Command head incidentally matches the
    /// `HEAD NAME BRACED BRACED` four-token shape but is **not** a
    /// proc-factory wrapper, so the signature scanner must not
    /// treat it as one.  Single source of truth for the analyser's
    /// former `_FACTORY_SKIP_HEADS` list (registered heads only;
    /// non-command heads like `method` / `itcl::class` are handled
    /// by a small residual set in the scanner).
    NotProcFactory => NOT_PROC_FACTORY;

    /// Command whose codegen always lowers to a dedicated runtime
    /// helper (or to a structured IR node) and never falls back to
    /// the interpreter — so a call to it needs no runtime frame in
    /// the callee.  The var-escape analysis treats only these as
    /// frame-free.  Single source of truth for the former
    /// `_FRAMELESS_RUNTIME_COMMANDS` allow-list.  Keep audited:
    /// stamping a command that secretly eval-falls-back would
    /// break eval-inside-proc semantics in escape-free procs.
    FramelessRuntime => FRAMELESS_RUNTIME;

    /// First argument is a variable *name* (read / write / modify),
    /// not a value — `set` / `incr` / `append` / `lappend` / `unset`.
    /// The var-escape analysis uses this to detect dynamic-name forms
    /// (`set $n value`). Single source of truth for the former
    /// `NAME_FIRST_COMMANDS` allow-list.
    FirstArgVarname => FIRST_ARG_VARNAME;

    /// A `VarRead`-role argument names the *whole* array, not a single
    /// element (`array` / `parray`), so a write to any element is
    /// observed by the read. Single source of truth for the former
    /// `WHOLE_ARRAY_COMMANDS` allow-list.
    WholeArrayArg => WHOLE_ARRAY_ARG;

    /// Executes a body through the interpreter, opening a
    /// name-resolution channel back into the local frame — `eval` /
    /// `uplevel` / `apply` / `source` / `namespace` / `interp`. The
    /// var-escape slot resolver treats a frame reached this way as
    /// hash-backed. Single source of truth for the former
    /// `DYNAMIC_EVAL_COMMANDS` allow-list. (Distinct from
    /// `HAS_INTERP_EVAL` / `CREATES_DYNAMIC_BARRIER`, which only some
    /// of these carry.)
    DynamicEvalBody => DYNAMIC_EVAL_BODY;

    /// (Subcommand) introspects program state by variable *name* —
    /// `info exists|vars|locals|args|default`. The var-escape slot
    /// resolver treats the named variable as ineligible for a slot.
    /// Single source of truth for the former
    /// `INFO_INTROSPECTING_SUBCMDS` list.
    IntrospectsByName => INTROSPECTS_BY_NAME;

    /// (Subcommand) targets a variable by *name* —
    /// `trace add|remove|info|variable|vdelete|vinfo`. Used by the
    /// var-escape slot resolver. Single source of truth for the former
    /// `TRACE_NAME_TARGETING` list.
    TargetsVariableByName => TARGETS_VARIABLE_BY_NAME;

    /// Aliases / reads / writes variables through the frame's *hash
    /// bucket* by name (`upvar` / `global` / `variable` / `lassign` /
    /// `lset` / `regexp` / `regsub` / `scan` / `binary` / `vwait` /
    /// `tkwait`), so a named variable it touches cannot live in an
    /// indexed slot. Single source of truth for the former
    /// `FRAME_HASH_BUILTINS` list.
    FrameHashBuiltin => FRAME_HASH_BUILTIN;

    /// A Tcl auto-loading / library proc that user code is expected to
    /// redefine (`unknown`, `auto_*`, `pkg_*`, `tclLog`,
    /// `tcl_findLibrary`, the `tcl_*Word*` helpers, …), so redefining
    /// it must not fire the W113 "overrides a built-in" warning.
    /// Single source of truth for the former
    /// `OVERRIDABLE_LIBRARY_PROCS` list.
    OverridableLibraryProc => OVERRIDABLE_LIBRARY_PROC;

    /// The WASM backend lowers this command to a structural construct
    /// that imports / emits no runtime helper of its own (`foreach`,
    /// `namespace`, `package`, `proc`).  Consulted by the WASM import
    /// collector.
    WasmEmitsNothing => WASM_EMITS_NOTHING;

    /// The registry's simple `min..=max` [`crate::Arity`] is a coarse
    /// floor/ceiling only — this command's real grammar is a clause
    /// chain validated by [`crate::spec::CommandSpec::clause_shape_check`]
    /// (`if`'s `elseif`/`else` chain). The generic arity floor/ceiling
    /// diagnostic (E002/E003) skips a command carrying this trait so its
    /// dedicated structural diagnostic owns arity together with clause
    /// shape — one precise diagnostic per malformed call instead of a
    /// redundant generic one alongside it.
    StructurallyCheckedArity => STRUCTURALLY_CHECKED_ARITY;

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
    ExprConcatenatesArgs => EXPR_CONCATENATES_ARGS;

    /// The command evaluates the concatenation of **every** trailing word
    /// from its first [`crate::arg_role::ArgRole::Body`] argument onwards
    /// as one script — the `Tcl_ConcatObj` eval family: `eval`, `uplevel`,
    /// `namespace eval`, `namespace inscope`, and `interp eval`. The
    /// script-side counterpart of [`Traits::EXPR_CONCATENATES_ARGS`].
    ///
    /// `Tcl_ConcatObj` (`generic/tclUtil.c`) takes each word's **string
    /// representation** — so a braced word contributes its *contents*, the
    /// outer braces already stripped by Tcl's word parsing — trims ASCII
    /// whitespace from each end of it, and joins the results with a single
    /// space. Commands therefore span word boundaries. tclsh8.6.14 and
    /// tclsh9.0.4 both confirm:
    ///
    /// ```text
    /// concat {set x} {5}      → set x 5
    /// concat "  a  " "  b  "  → a b
    /// eval set l2 hello       → sets l2 to "hello"
    /// eval {set l2} hello     → sets l2 to "hello"
    /// ```
    ///
    /// **Consumer contract.** A consumer that walks `ArgRole::Body`
    /// arguments must never treat the first Body-role word as the whole
    /// script when this trait is present and further words follow it.
    /// Analysing `eval set l2 hello` as the one-word script `set` invents a
    /// wrong-#-args error and loses the write to `l2`. Either join the
    /// trailing words per the rule above (sound only when every one of them
    /// is a static literal — a bare or braced word with no `$` / `[`
    /// substitution) or decline to walk the command at all.
    ///
    /// **Limit.** The join is a *static* approximation. One dynamic word
    /// makes the whole script unknowable at analysis time, because
    /// substitution happens before concatenation: `eval $cmd arg` may run
    /// any command at all. Consumers must consume such a call without
    /// walking rather than guess.
    ///
    /// `catch` is deliberately **not** in this family: it takes a single
    /// bounded script argument, so its remaining words are result/options
    /// variable names, not script text.
    ///
    /// [`Traits::SCRIPT_APPENDS_LIST_ARGS`] refines the join rule for the
    /// one family member that does not space-join.
    ScriptConcatenatesArgs => SCRIPT_CONCATENATES_ARGS;

    /// Refines [`Traits::SCRIPT_CONCATENATES_ARGS`] for the one family
    /// member whose trailing words are appended as **list elements**
    /// instead of being space-joined into the script text —
    /// `namespace inscope`.
    ///
    /// `namespace inscope ns script ?arg ...?` is equivalent to
    /// `namespace eval ns [concat script [list arg ...]]`
    /// (`NamespaceInscopeCmd`, `generic/tclNamesp.c`), so each trailing word
    /// arrives at the invoked command as exactly one argument, whatever
    /// whitespace it contains. tclsh8.6.14 and tclsh9.0.4 both confirm the
    /// divergence:
    ///
    /// ```text
    /// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
    /// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
    /// ```
    ///
    /// A consumer must therefore **not** space-join a call carrying this
    /// trait. Reconstructing the real script needs list quoting the analyser
    /// does not model, so the sound response to any trailing word here is to
    /// consume the command without walking it.
    ScriptAppendsListArgs => SCRIPT_APPENDS_LIST_ARGS;

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
    EstablishesVariableTrace => ESTABLISHES_VARIABLE_TRACE;

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
    TransfersControl => TRANSFERS_CONTROL;

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
    FireAndForgetTeardown => FIRE_AND_FORGET_TEARDOWN;

    /// Command is a math-operator head (`+`, `eq`, `ne`, `in`, `ni`,
    /// and the `tcl::mathop::*` spellings): a real callable command in
    /// every dialect whose profile has `operators_as_commands`, but
    /// *never* a command head where operators live only inside `expr`
    /// (F5 iRules; `tk` when modelled). Replaces the retired
    /// `NON_IRULES_OPERATORS` membership tag as the operator-head
    /// marker (dialect-profile-model.md §9, Milestone 5).
    OperatorCommand => OPERATOR_COMMAND;

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
    TclooNextChain => TCLOO_NEXT_CHAIN;

    /// `TclOO` `my` — dispatches a method on the **current object**, from
    /// inside that object's own method, constructor, or destructor
    /// (`TclOOMyObjCmd`, `generic/tclOO.c`). Unlike calling the object's
    /// public command, `my` reaches non-exported (and, from 9.0, private)
    /// methods, so a `my foo` word is a real reference to the enclosing
    /// class's `foo` method and must resolve, rename, and report references
    /// as one.
    ///
    /// One of the three `TclOO` method-context keyword traits, which sit on
    /// distinct axes and must not be conflated by a consumer:
    ///
    /// - **`TCLOO_SELF_DISPATCH`** (`my`) — instance self-dispatch: the
    ///   first argument names a method on *this* object.
    /// - [`Traits::TCLOO_NEXT_CHAIN`] (`next` / `nextto`) — superclass
    ///   chain: no method name at all, the callee is the next
    ///   implementation of the *currently executing* method.
    /// - [`Traits::TCLOO_INTROSPECTION`] (`self`) — introspection, never
    ///   dispatch: it returns a description of the current invocation.
    ///
    /// `link` is deliberately outside all three. `link` *creates* bareword
    /// commands in the object's namespace (`link {alias method}`), so those
    /// barewords are per-class data — not language keywords — and no
    /// consumer may treat `link` itself as a dispatch site (issue #1026).
    ///
    /// A dialect that gained or lost `my` would propagate through this
    /// spec's `dialects` mask, so consumers query the registry rather than
    /// spelling the name: see
    /// [`crate::registry::CommandRegistry::method_dispatch_keyword`].
    TclooSelfDispatch => TCLOO_SELF_DISPATCH;

    /// `TclOO` `self` — **introspects** the current method invocation
    /// (`TclOOSelfObjCmd`, `generic/tclOO.c`): which object is running,
    /// which method, which declaring class, which instance namespace, and
    /// where in the call chain it sits. It dispatches nothing.
    ///
    /// The distinction matters to every consumer that walks method bodies
    /// looking for method references: a `self` word introduces no method
    /// name, so a consumer that lumps it in with
    /// [`Traits::TCLOO_SELF_DISPATCH`] would treat `self class` as a call
    /// to a method named `class`. Its argument is a closed subcommand set
    /// (`arg_values` / `closed_value_args` on the spec), not a method name.
    ///
    /// See [`Traits::TCLOO_SELF_DISPATCH`] for the full family and
    /// [`crate::registry::CommandRegistry::method_dispatch_keyword`] for
    /// the query consumers use instead of a literal name test.
    TclooIntrospection => TCLOO_INTROSPECTION;

    /// Each body argument runs only when this command's own run-time
    /// selection picks it, and the command performs no iteration — `if`
    /// (at most one clause body plus an optional `else`) and `try` (handler
    /// bodies reached only on the matching exception, with `finally` the one
    /// exception that always runs).
    ///
    /// The fact a consumer needs is that nothing established inside such a
    /// body **dominates** the code after the command: a `package require`
    /// there is conditional, and a variable written there is not reliably
    /// set afterwards.
    ///
    /// Deliberately narrower than [`Traits::CONTROL_FLOW`], which also
    /// covers `while` / `for` / `foreach` / `lmap`. A loop body is
    /// *repeatable* as well as skippable, so the two questions have
    /// different answers and different consumers —
    /// `Analyser::control_flow_body_depth` (straight-line-ness, driven by
    /// `CONTROL_FLOW`) and `Analyser::conditional_depth` (domination, driven
    /// by this trait) must not be merged. Also distinct from
    /// [`Traits::HAS_BOOLEAN_COND`], which is about an argument being read
    /// as a boolean expression (`if` / `while` / `for`) rather than about
    /// which bodies run.
    BranchSelectedBody => BRANCH_SELECTED_BODY;

    /// Raises a *catchable* exception — completes `TCL_ERROR`, which
    /// `catch` / `try` intercept: `error` (`Tcl_ErrorObjCmd`,
    /// `generic/tclCmdAH.c`) and `throw` (`TclNRThrowObjCmd`,
    /// `generic/tclBasic.c`, 8.6+). The strict subset of
    /// [`Traits::TERMINATES_BLOCK`] that sources an enclosing `try`'s
    /// on-error edge: `exit` (`Tcl_ExitObjCmd`) terminates the
    /// process rather than unwinding through exception ranges, and
    /// `return` pops the frame with `TCL_RETURN`, so neither is a
    /// throw point. Consumed by the CFG builder's throw-block
    /// recording.
    CatchableThrow => CATCHABLE_THROW;

    /// Jumps to the innermost enclosing loop's *post-loop* target —
    /// `break`, which completes `TCL_BREAK` (`Tcl_BreakObjCmd`,
    /// `generic/tclCmdAH.c`); inside a compiled loop the bytecode
    /// compiler rewrites it into a jump to the loop's break fixup
    /// site (`TclCompileBreakCmd`, `generic/tclCompCmds.c`).
    /// Loop-jump classification for the CFG builder's
    /// `break`/`continue` edge lowering and the inline emitters'
    /// straight-line-body gate; paired with
    /// [`Traits::CONTINUES_LOOP`].
    BreaksLoop => BREAKS_LOOP;

    /// Jumps to the innermost enclosing loop's *next-iteration*
    /// target — `continue`, which completes `TCL_CONTINUE`
    /// (`Tcl_ContinueObjCmd`, `generic/tclCmdAH.c`;
    /// `TclCompileContinueCmd`, `generic/tclCompCmds.c`). The
    /// other half of the loop-jump classification — see
    /// [`Traits::BREAKS_LOOP`].
    ContinuesLoop => CONTINUES_LOOP;

    /// Replaces the current procedure's frame — `tailcall`
    /// (`TclNRTailcallObjCmd`, `generic/tclBasic.c`, 8.6+), which
    /// schedules its command to run after the frame pops and always
    /// completes `TCL_RETURN`, so control never resumes in the
    /// calling body. Deliberately *not* [`Traits::TERMINATES_BLOCK`]:
    /// the analysis CFG promotes it to a proc-exit terminator, but
    /// codegen must keep the plain fall-through call shape so the
    /// emitted bytecode matches C Tcl's.
    ReplacesFrame => REPLACES_FRAME;

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
    SafeInterpHidden => SAFE_INTERP_HIDDEN;

    /// Declares the enclosing file to be a loadable **package**, so the
    /// commands it defines are public API that files this compilation unit
    /// cannot see may call — `package provide`, `package ifneeded`.  The
    /// interprocedural call-site seed (`tcl_compiler::unit_scope`) refuses
    /// to treat a file's visible call sites as the complete caller set once
    /// this appears, because no project enumeration bounds a consumer in
    /// another checkout (issue #977).
    ProvidesPackage => PROVIDES_PACKAGE;

    /// Pulls another compilation unit's script into *this* interpreter, so
    /// that unit's code can call back into the commands this file defines —
    /// `source`, `load`, `package require`, `auto_load`, `auto_import`.  A
    /// weaker signal than [`Traits::PROVIDES_PACKAGE`]: the loaded unit is
    /// normally a file the host's project already contains, so real
    /// cross-file evidence can cover it (issue #977).
    LoadsExternalUnit => LOADS_EXTERNAL_UNIT;

    /// Publishes a command name for another unit to import or dispatch
    /// through — `namespace export`, `namespace ensemble create` /
    /// `configure`.  Like [`Traits::PROVIDES_PACKAGE`], it marks the file's
    /// commands as an API surface whose callers the file does not contain
    /// and no enumeration bounds (issue #977).
    ExportsCommand => EXPORTS_COMMAND;

    /// The interpreter's fallback for a command word that resolves to
    /// nothing — `unknown`.  Tcl dispatches *every* unresolved command
    /// word to it, passing the word itself followed by that call's
    /// arguments (tclsh8.6/9.0-confirmed: `bogus beta gamma` with a
    /// user-defined `proc unknown {cmd args}` in scope runs the handler
    /// with `cmd` = `bogus`, `args` = `beta gamma`).
    ///
    /// The interprocedural call-site seed (`tcl_compiler::unit_scope`)
    /// consults this trait so it can enumerate those dispatches as real
    /// call sites of a module's own handler instead of seeing only its
    /// direct callers — a coincidentally-uniform set of which would
    /// otherwise fold a genuinely runtime-varying parameter (issue #1044).
    /// Carried by the spec so no consumer spells the name `unknown`.
    ///
    /// Global only: a namespace-local `proc unknown` is *not* the handler
    /// for unresolved words in that namespace — tclsh8.6/9.0 dispatch to
    /// `::unknown` regardless of the calling namespace. (`namespace
    /// unknown NAME` registers a per-namespace handler explicitly; that
    /// path is already modelled by its handler argument's
    /// [`crate::arg_role::ArgRole::CommandPrefix`] role.)
    UnresolvedCommandHandler => UNRESOLVED_COMMAND_HANDLER;

    /// Evaluates its script argument in a **different stack frame** than
    /// the one the call is written in — `uplevel`, whose body runs in the
    /// frame the level argument selects (`Tcl_UplevelObjCmd`,
    /// `generic/tclProc.c`).
    ///
    /// Distinct from [`Traits::EVALUATES_CODE`], which every `eval`-family
    /// command carries: `eval`, `catch`, and `namespace inscope` bodies all
    /// run somewhere, but only this one runs somewhere the *writing* frame
    /// does not describe. And distinct from
    /// [`crate::arg_role::ArgRole::Name`]-qualified commands like
    /// `namespace eval`, which shift *namespace* while staying in the same
    /// frame.
    ///
    /// A consumer that walks `ArgRole::Body` arguments with the enclosing
    /// unit's own context must skip these bodies: the context is wrong, so
    /// every bare command word in the body resolves against the wrong
    /// namespace. `tcl_compiler::unit_scope` has a dedicated scan
    /// (`upframe_scan_bodies`) that visits them with the frame the level
    /// argument actually selects.
    ///
    /// tclsh8.6/9.0-confirmed: inside `::foo::runIt`, `uplevel #0 { helper
    /// b }` calls `::helper`, never the `::foo::helper` sitting in the
    /// enclosing namespace.
    EvaluatesInShiftedFrame => EVALUATES_IN_SHIFTED_FRAME;
}

/// Every trait that widens a file's caller set beyond the file itself — the
/// union [`crate::registry::CommandRegistry::unit_linkage`] reports, and that
/// `tcl_compiler::unit_scope` gates the interprocedural call-site seed on.
///
/// Kept beside the declarations so a newly-stamped linkage trait joins the
/// union here rather than needing a second edit in the consumer.
pub const UNIT_LINKAGE_TRAITS: Traits = Traits::PROVIDES_PACKAGE
    .union(Traits::LOADS_EXTERNAL_UNIT)
    .union(Traits::EXPORTS_COMMAND);

/// A `Trait`'s bit is `1 << discriminant`, so the set must fit the `u128`.
/// Unlike a collision — which the enum makes unrepresentable — running out of
/// width is a real limit worth failing the build over.
const _: () = assert!(
    Trait::ALL.len() <= 128,
    "more traits than a u128 bitset can hold — widen `Traits` to [u64; N] via a custom bit type"
);

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
