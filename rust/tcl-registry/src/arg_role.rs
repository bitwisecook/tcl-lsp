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
    /// A `format` %-string — the printf-style mini-language whose
    /// conversions are version-gated (the §6 argument-DSL rung of the
    /// dialect-profile model; `%b` is 8.6+, `%ll…u` is 9.0+).
    FormatString,
    /// A `scan` %-string — same rung as [`ArgRole::FormatString`], with
    /// scan's conversion set (`%b` is 8.6+).
    ScanFormat,
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
    /// A bare command **name** held as a data argument — introspected or
    /// manipulated, never invoked here.  Unlike [`Self::CommandPrefix`] the
    /// word is the *whole* name (no arguments are appended and no callback
    /// arity applies): `info body PROC` / `info args PROC` / `info default
    /// PROC` read a proc, `namespace origin NAME` resolves one.  Like
    /// `CommandPrefix` it is a first-class command **reference** — the
    /// compiler records the word as a call site so find-references /
    /// go-to-definition / rename / call-hierarchy reach the named command —
    /// but it carries no arity to check.  The named command **must exist**
    /// for the introspection to succeed (C errors otherwise), so the
    /// reference participates in the W123 unresolved-command pass.
    CommandName,
    /// A bare command name held as data by an existence **probe** — the
    /// command legitimately may not exist (`namespace which -command NAME`
    /// returns `""` for an unknown name; an exact, pattern-free
    /// `info commands NAME` returns an empty list).  Identical navigation
    /// semantics to [`Self::CommandName`] — the word is a first-class,
    /// exactly-writable command reference for find-references /
    /// go-to-definition / rename — but the **existence policy differs**:
    /// the reference must never feed the W123 unresolved-command pass
    /// (issue #945 fault 9: reference identity and existence assertion are
    /// orthogonal, and a probe asserts nothing).
    CommandNameProbe,
    /// An anonymous-lambda **literal** — Tcl's `apply` argument shape, a
    /// `{argList body ?namespace?}` list rather than a plain script.
    ///
    /// Distinct from [`Self::Body`]: a `Body`-role argument *is* the script
    /// directly, walked by re-segmenting its own text. A `LambdaLiteral`
    /// argument is one level removed — it is a 2- or 3-element Tcl **list**
    /// whose element 0 is a parameter list ([`Self::ParamList`] shape, not
    /// code) and element 1 is the actual body script; an optional element 2
    /// names the namespace the body runs in. A consumer that recurses
    /// [`Self::Body`] arguments directly would mis-segment the whole
    /// `{argList} {body}` pair as if it were script source. Every command
    /// declaring this role is dispatched identically by
    /// [`Self::carries_script`] callers: split the list, walk element 0 as a
    /// parameter list and element 1 as a body, leave the rest verbatim.
    ///
    /// `apply` is the only built-in with this shape today, but the role is
    /// named for the *shape*, not the command — a future command sharing it
    /// (or `apply` reached indirectly, e.g. quoted through `[list apply
    /// {…} $x]`) is recognised the same way, with no command-name check in
    /// the consumer.
    LambdaLiteral,
    /// A word that names a **namespace** — `namespace children ::tomato`,
    /// `namespace exists ::x`, `namespace delete ::a ::b`, `namespace eval
    /// NS body`, `namespace inscope NS script`, `namespace upvar NS v local`.
    ///
    /// A first-class namespace **reference**: the compiler records the word
    /// as a namespace occurrence so go-to-definition / hover / find-references
    /// reach the `namespace eval` block(s) that declare it (issue #1088).
    /// Namespaces are their own symbol space in Tcl — disjoint from commands
    /// and from variables — so this is neither
    /// [`Self::CommandName`] nor [`Self::VarRead`]; a consumer that treated a
    /// namespace word as either would resolve `namespace exists ::tomato` to
    /// an unrelated same-named proc.
    ///
    /// The word may be written **relative**, and then it roots against the
    /// call site's own current namespace, exactly as Tcl resolves it
    /// (tclsh 9.0.4 and 8.6.16, byte-identical: inside `namespace eval
    /// ::outer`, `namespace exists inner` finds `::outer::inner` and the
    /// global `namespace exists inner` does not).
    ///
    /// Deliberately **not** stamped on positions that merely *look* like
    /// namespace words: `namespace qualifiers` / `namespace tail` take an
    /// arbitrary string (`namespace tail a::b` answers `b` whether or not
    /// `::a` exists), `namespace import` / `export` / `forget` take glob
    /// **patterns**, `namespace origin` / `which` take command names, and
    /// `namespace code` takes a script.
    ///
    /// Whether the named namespace has to *exist* is the subcommand's
    /// business, not the role's: `namespace exists` probes (`0` for an
    /// unknown name), while `namespace children` / `delete` error
    /// (`namespace "::never" not found`, rc 1, on both interpreters). The
    /// role carries navigation identity only, the same split
    /// [`Self::CommandName`] / [`Self::CommandNameProbe`] draw for commands.
    NamespaceName,
    /// A word consumed **purely as a boolean** — the command runs it through
    /// `Tcl_GetBooleanFromObj` (or the Tcl-level equivalent) and its bytes
    /// are never otherwise observable, so every accepted spelling of the same
    /// truth value is interchangeable (issue #1256).
    ///
    /// That interchangeability is the whole point: it is what lets the
    /// formatter's canonical-boolean rewrite turn `-strict yes` into
    /// `-strict true` without changing behaviour, and what lets completion
    /// and hover offer the boolean vocabulary
    /// ([`crate::abbrev::BOOLEAN_KEYWORDS`]) at the position. Before this
    /// role the fact had to be *inferred* from an option's declared value set
    /// happening to enumerate the boolean words, which covered two options in
    /// the whole registry.
    ///
    /// Stamp it only where the bytes truly do not escape. A value that is
    /// stored and later compared as a string (`set flag yes` … `$flag eq
    /// "yes"`), or echoed back by an introspection subcommand, is **not** in
    /// this role — `true` and `yes` are different strings even though both
    /// are truthy.
    ///
    /// tclsh-proof (8.6.16 / 9.0.4): `fconfigure $c -blocking` accepts every
    /// spelling and reports back the canonical integer —
    /// `chan configure $c -blocking yes; chan configure $c -blocking` → `1`,
    /// and `-blocking tru` → `1` as well (prefix matching, per
    /// [`crate::abbrev::boolean_table`]).
    Boolean,
    /// A word consumed as **either** a boolean or a number — `0` and `1` are
    /// valid integers as well as booleans, so a consumer cannot tell which
    /// language the author meant and must leave the bytes alone.
    ///
    /// Distinct from [`Self::Boolean`] precisely so a rewriting consumer
    /// abstains here by construction rather than by a guess: the formatter's
    /// canonical-boolean rewrite would turn a deliberate `1` into `true` at a
    /// position where the command reads a count.
    NumericOrBoolean,
    /// The word that becomes the command's **own result** — `return $w`
    /// completes the enclosing procedure or method with `$w`.
    ///
    /// The role exists so an analysis can *prove* what a body hands back
    /// without naming `return`: the only consumer today is the metaclass
    /// `unknown`-dispatch proof (issue #1303), which must establish that
    /// `[Widget .w]` yields `.w` rather than assume it.
    ///
    /// Stamp it only on a word whose bytes really are the result, unchanged.
    /// `return`'s own resolver therefore emits it **only** for the bare
    /// one-word form: `return -code error $msg` completes exceptionally with
    /// `$msg` as an error payload, and `return -level 0 $v` completes the
    /// *caller* — neither is "this command's result is that word", so neither
    /// carries the role.
    Result,
}

impl ArgRole {
    /// Every variant, so a consumer can assert it handles the whole space
    /// rather than the subset it happened to think of.
    pub const ALL: &'static [Self] = &[
        Self::Body,
        Self::Expr,
        Self::VarWrite,
        Self::VarRead,
        Self::LoopVarList,
        Self::ParamList,
        Self::Name,
        Self::Pattern,
        Self::Option,
        Self::Value,
        Self::Subcommand,
        Self::OptionTerminator,
        Self::FormatString,
        Self::ScanFormat,
        Self::Channel,
        Self::Index,
        Self::Keyword,
        Self::CommandPrefix,
        Self::CommandName,
        Self::CommandNameProbe,
        Self::LambdaLiteral,
        Self::NamespaceName,
        Self::Result,
        Self::Boolean,
        Self::NumericOrBoolean,
    ];

    /// Whether a word in this role is consumed **purely as a boolean**, so
    /// every accepted spelling of the same truth value is interchangeable.
    ///
    /// The one place that answers "is this word consumed as a boolean" —
    /// the formatter's canonical-boolean rewrite, completion's value list,
    /// and hover all read it, so they cannot drift apart. Deliberately
    /// `false` for [`Self::NumericOrBoolean`]: that position also accepts an
    /// integer, and `0`/`1` belong to both languages.
    #[must_use]
    pub const fn consumes_boolean(self) -> bool {
        matches!(self, Self::Boolean)
    }

    /// Whether an argument in this role carries **executable Tcl** that a
    /// consumer walking the code must descend into.
    ///
    /// The one place that answers this question. Every walker that recurses
    /// into code — the semantic-token walker, the iRules object-reference
    /// walker — reads it, so they cannot drift apart about what counts as
    /// executable.
    ///
    /// The match is exhaustive on purpose: a new [`ArgRole`] that can hold a
    /// script fails to compile until someone decides which side it falls on.
    /// That decision used to be implicit, and the walkers each carried their
    /// own idea of it — which is how an object referenced only from a `switch`
    /// arm came to be invisible to the reference graph that `bigip-cleanup`
    /// decides deletions from. A clause list is not an [`ArgRole::Body`], so
    /// nothing descended into it (see [`crate::CommandSpec::case_list`], which
    /// carries the scripts a role cannot).
    ///
    /// [`ArgRole::Body`] is a complete script. [`ArgRole::Expr`] is not, but the
    /// `[…]` substitutions inside it are, and they run with the same effects a
    /// body's would — so both are walked.
    ///
    /// [`ArgRole::CommandPrefix`] is deliberately *not* script-bearing: its
    /// first word is a callable **reference**, not code. Recursing it would read
    /// a bareword proc name as a script.
    #[must_use]
    pub const fn carries_script(self) -> bool {
        match self {
            Self::Body | Self::Expr | Self::LambdaLiteral => true,
            Self::CommandPrefix
            | Self::CommandName
            | Self::CommandNameProbe
            | Self::VarWrite
            | Self::VarRead
            | Self::LoopVarList
            | Self::ParamList
            | Self::Name
            | Self::Pattern
            | Self::Option
            | Self::Value
            | Self::Subcommand
            | Self::OptionTerminator
            | Self::FormatString
            | Self::ScanFormat
            | Self::Channel
            | Self::Index
            | Self::NamespaceName
            | Self::Result
            | Self::Boolean
            | Self::NumericOrBoolean
            | Self::Keyword => false,
        }
    }

    /// Whether an argument in this role **is** a variable name — the word's
    /// own text names a cell the command reads or writes, rather than being a
    /// value, a script, a pattern, or a command reference.
    ///
    /// The one place that answers this question, so the consumers that need
    /// it cannot drift apart: the analyser's reference recorder (a `VarRead`
    /// word is a genuine use site of the named cell — `puts [set m]` /
    /// `info exists m` read `m` exactly as `$m` does), the dead-store
    /// suppressor's command-substitution scan (a **braced** word in this role
    /// is a *literal* name, so a `$x` inside it is part of that name and not
    /// a read of `x` — issue #1109), and the cursor resolver that answers
    /// which cell a brace-quoted name word denotes (issue #1108).
    ///
    /// [`Self::LoopVarList`] is deliberately excluded: that word is a *list*
    /// of names, not one name, so a consumer must split it before it has a
    /// variable name at all. The match is exhaustive on purpose — a new role
    /// that names a variable fails to compile until someone decides.
    #[must_use]
    pub const fn names_variable(self) -> bool {
        match self {
            Self::VarWrite | Self::VarRead => true,
            Self::Body
            | Self::Expr
            | Self::LambdaLiteral
            | Self::CommandPrefix
            | Self::CommandName
            | Self::CommandNameProbe
            | Self::LoopVarList
            | Self::ParamList
            | Self::Name
            | Self::Pattern
            | Self::Option
            | Self::Value
            | Self::Subcommand
            | Self::OptionTerminator
            | Self::FormatString
            | Self::ScanFormat
            | Self::Channel
            | Self::Index
            | Self::NamespaceName
            | Self::Result
            | Self::Boolean
            | Self::NumericOrBoolean
            | Self::Keyword => false,
        }
    }

    /// Whether a **brace-quoted** word in this role is nonetheless evaluated —
    /// as script or as an expression — in the **calling frame**, so a `$name`
    /// written inside it is a genuine read at this call site.
    ///
    /// Tcl performs no substitution on a braced word: `puts {$y}` prints the
    /// two characters `$y` and reads nothing (tclsh 9.0.4 / 8.6.16 agree).
    /// Whether the *callee* then evaluates that text, and in whose frame, is
    /// a per-role fact — which is exactly what this answers, and the one
    /// place that answers it. `expr {$a + $b}` and `if {$c} {…}` re-evaluate
    /// their braced word where the caller's variables are in scope, so the
    /// names inside are read here and now. Everything else — a pattern, a
    /// format string, a channel, a plain value, a deferred command prefix —
    /// consumes the word as inert data, or evaluates it later, elsewhere, or
    /// never.
    ///
    /// [`Self::LambdaLiteral`] is `false` on purpose: `apply {{} {puts $x}}`
    /// runs in a **fresh** frame, so nothing inside it reads the caller's
    /// `x` (tclsh 9.0.4 / 8.6.16: `set x 7; apply {{} {puts $x}}` errors
    /// with `can't read "x": no such variable`).
    ///
    /// The match is exhaustive on purpose: a new role that carries
    /// caller-frame code fails to compile until someone decides which side
    /// it falls on.
    #[must_use]
    pub const fn braced_word_evaluated_in_frame(self) -> bool {
        match self {
            Self::Body | Self::Expr => true,
            Self::LambdaLiteral
            | Self::CommandPrefix
            | Self::CommandName
            | Self::CommandNameProbe
            | Self::VarWrite
            | Self::VarRead
            | Self::LoopVarList
            | Self::ParamList
            | Self::Name
            | Self::Pattern
            | Self::Option
            | Self::Value
            | Self::Subcommand
            | Self::OptionTerminator
            | Self::FormatString
            | Self::ScanFormat
            | Self::Channel
            | Self::Index
            | Self::NamespaceName
            | Self::Result
            | Self::Boolean
            | Self::NumericOrBoolean
            | Self::Keyword => false,
        }
    }
}

/// A canonical finite set of exact callback argument counts.
///
/// Used when one registry-declared callback site can invoke the same prefix
/// with several non-contiguous arities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppendedAritySet {
    counts: &'static [u8],
}

impl AppendedAritySet {
    /// Build a non-empty, strictly increasing set of exact appended counts.
    ///
    /// Keeping construction checked makes equality, hashing, iteration, and
    /// diagnostics deterministic without asking generic consumers to
    /// normalise registry data.
    #[must_use]
    pub const fn from_sorted_unique(counts: &'static [u8]) -> Self {
        assert!(!counts.is_empty(), "an appended-arity set cannot be empty");
        let mut index = 1;
        while index < counts.len() {
            assert!(
                counts[index - 1] < counts[index],
                "appended arities must be strictly increasing"
            );
            index += 1;
        }
        Self { counts }
    }

    /// The possible exact appended counts, in increasing order.
    #[must_use]
    pub const fn counts(self) -> &'static [u8] {
        self.counts
    }
}

/// How many arguments a command appends to a [`ArgRole::CommandPrefix`]
/// callback when it invokes it.
///
/// Sourced from C Tcl behaviour (`lsort -command` appends 2, `trace add
/// variable` appends 3, `socket -server` appends 3, …). Paired with a
/// `CommandPrefix` declaration so the arity checker can validate that the
/// referenced proc accepts `baked_args + appended` arguments, where
/// `baked_args` are any words already present in the prefix itself
/// (`{myCmp extra}` bakes one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AppendedArity {
    /// Exactly `n` args are appended (`lsort -command` → `Exactly(2)`).
    Exactly(u8),
    /// One count from a finite, non-contiguous set is appended. Every count in
    /// the set must be accepted by a statically checked callback (`trace add
    /// execution` with both `enter` and `leave` → `OneOf({2, 4})`).
    OneOf(AppendedAritySet),
    /// At least `n` args are appended; the maximum is unbounded or
    /// form-dependent (`regsub -command` → `AtLeast(1)`, variadic `interp
    /// alias` → `AtLeast(0)`).
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
            Self::OneOf(set) => set.counts()[0],
            Self::Unknown => 0,
        }
    }

    /// The maximum number of appended args, or `None` when unbounded /
    /// unknown.
    #[must_use]
    pub const fn max(self) -> Option<u8> {
        match self {
            Self::Exactly(n) => Some(n),
            Self::OneOf(set) => Some(set.counts()[set.counts().len() - 1]),
            Self::AtLeast(_) | Self::Unknown => None,
        }
    }

    /// Whether an arity check should run at all — `false` for
    /// [`Unknown`](Self::Unknown), whose count is indeterminate.
    #[must_use]
    pub const fn is_checkable(self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// Iterate the finite exact alternatives, if this descriptor has them.
    ///
    /// `AtLeast` is an open range and `Unknown` deliberately carries no
    /// checkable alternatives, so both return `None`.
    #[must_use]
    pub fn exact_counts(self) -> Option<impl Iterator<Item = u8>> {
        let (single, alternatives) = match self {
            Self::Exactly(count) => (Some(count), &[][..]),
            Self::OneOf(set) => (None, set.counts()),
            Self::AtLeast(_) | Self::Unknown => return None,
        };
        Some(single.into_iter().chain(alternatives.iter().copied()))
    }
}

#[cfg(test)]
mod tests {
    use super::{AppendedArity, AppendedAritySet, ArgRole};

    #[test]
    fn finite_appended_arity_sets_preserve_non_contiguous_counts() {
        let arity = AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 4]));
        assert_eq!(arity.min(), 2);
        assert_eq!(arity.max(), Some(4));
        assert_eq!(
            arity.exact_counts().unwrap().collect::<Vec<_>>(),
            vec![2, 4]
        );
    }

    /// `braced_word_evaluated_in_frame` is the one answer to "does the callee
    /// substitute this brace-quoted word against the *caller's* variables?".
    /// Only the two roles that re-evaluate their word as caller-frame code say
    /// yes; every other role consumes the word as data, defers it, or runs it
    /// in a fresh frame.
    ///
    /// tclsh-proof: tclsh8.6.14 — `puts {$y}` prints `$y` with `y` undefined,
    /// while `expr {$y}` errors with `can't read "y": no such variable`.
    #[test]
    fn braced_word_evaluated_in_frame_is_body_and_expr_only() {
        let evaluated: Vec<ArgRole> = ArgRole::ALL
            .iter()
            .copied()
            .filter(|r| r.braced_word_evaluated_in_frame())
            .collect();
        assert_eq!(evaluated, vec![ArgRole::Body, ArgRole::Expr]);
    }

    /// A lambda literal carries code, but in a **fresh** frame, so it must not
    /// claim a read of the caller's variables.
    /// tclsh-proof: tclsh8.6.14 — `set x 7; apply {{} {puts $x}}` errors with
    /// `can't read "x": no such variable`.
    #[test]
    fn lambda_literal_is_script_bearing_but_not_caller_frame() {
        assert!(ArgRole::LambdaLiteral.carries_script());
        assert!(!ArgRole::LambdaLiteral.braced_word_evaluated_in_frame());
    }
}
