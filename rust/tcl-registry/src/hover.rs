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

//! Documentation and completion metadata for LSP features.

use crate::abbrev::Keyword;
use crate::arg_role::{AppendedArity, ArgRole};
use crate::body_kind::BodyKind;
use crate::lifecycle::{Lifecycle, LifecycleState};
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::{surface_admits};

/// Short hover content derived from man pages or vendor docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverSnippet {
    /// One-line summary.
    pub summary: &'static str,
    /// Invocation synopsis lines (e.g. `"for start test next body"`).
    pub synopsis: &'static [&'static str],
    /// Extended description.
    pub snippet: &'static str,
    /// Documentation source (e.g. `"Tcl for(1)"`).
    pub source: &'static str,
    /// Usage examples.
    pub examples: &'static str,
    /// Return value description.
    pub return_value: &'static str,
}

impl HoverSnippet {
    /// A hover with only summary, synopsis, and source — the common case.
    #[must_use]
    pub const fn brief(
        summary: &'static str,
        synopsis: &'static [&'static str],
        source: &'static str,
    ) -> Self {
        Self {
            summary,
            synopsis,
            snippet: "",
            source,
            examples: "",
            return_value: "",
        }
    }
}

/// Completion and hover metadata for a positional argument value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentValueSpec {
    /// Completable value text.
    pub value: &'static str,
    /// Short description in the completion list.
    pub detail: &'static str,
}

/// Outcome of a dynamically-computed option value — both how many argv
/// words it spans and whether what's there is actually valid.
///
/// `words` is reported even when `invalid` is `Some`: a scanning consumer
/// (arity counting, semantic tokens, completion) still needs to skip past
/// the value, whether or not it turns out to be well-formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionValueOutcome {
    /// Words consumed, valid or not.
    pub words: usize,
    /// `Some(msg)` when the value is invalid — the message to report.
    pub invalid: Option<&'static str>,
}

/// Computes how many words an option's value spans, and whether it's
/// valid, from the args starting at the value's position. `start` is the
/// 0-based index into `args` of the first value word.
pub type OptionValueHook = fn(args: &[&str], start: usize) -> OptionValueOutcome;

/// When a script-valued argument is evaluated relative to the invocation that
/// receives it.
///
/// This is deliberately separate from [`BodyKind`]: that descriptor answers
/// *which frame* a body uses, while this one answers *when* it runs.  The
/// distinction matters for callback options: a Tk widget constructor stores
/// `-command` for a later event, whereas `tcltest::test -body` evaluates its
/// script before the `test` invocation returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTiming {
    /// The script may run before the receiving invocation returns.
    SameInvocation,
    /// The receiving invocation stores the script for a later callback.
    Deferred,
    /// The invocation identifies executable text but neither evaluates nor
    /// stores it.  Removal/query forms use this for a command prefix that is
    /// matched against an existing registration (`trace remove`), so
    /// navigation still sees the reference without inventing a callback or a
    /// current-invocation control-flow edge.
    ReferenceOnly,
}

/// A value substituted into a stored callback immediately before Tcl evaluates
/// it, whose bytes originate outside the program.
///
/// Tk's `%` replacement language is deliberately modelled as data rather than
/// a `Body` role: the receiving command stores a script, then substitutes only
/// the declared markers when an event or validation actually fires.  The
/// registry lists *only* externally controlled markers.  Stable metadata such
/// as `%W` (widget pathname), `%d` (validation action), `%i` (index), and `%V`
/// (validation reason) is intentionally absent, so a callback cannot become a
/// taint source merely because it observes framework bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallbackTaintInput {
    /// A Tk `%` substitution.  The spelling includes the leading percent,
    /// for example `"%P"` or `"%A"`.
    TkPercent(&'static str),
}

impl CallbackTaintInput {
    /// The proposed value supplied to an entry/spinbox validation callback.
    pub const TK_PROPOSED_VALUE: Self = Self::TkPercent("%P");
    /// The value before a validation edit.
    pub const TK_CURRENT_VALUE: Self = Self::TkPercent("%s");
    /// Text being inserted or deleted by a validation edit.
    pub const TK_EDIT_TEXT: Self = Self::TkPercent("%S");
    /// The character delivered by a Tk key event.
    pub const TK_EVENT_CHAR: Self = Self::TkPercent("%A");
    /// The symbolic keysym delivered by a Tk key event. Shipped Tk specs treat
    /// this as metadata, but dialect packs may opt in when their threat model
    /// treats symbolic choices as tainted input.
    pub const TK_EVENT_KEYSYM: Self = Self::TkPercent("%K");
    /// The source spelling recognised in a callback script.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::TkPercent(spelling) => spelling,
        }
    }

    /// Whether this input occurs as an actual Tk `%` replacement in `text`.
    ///
    /// `%%` is Tk's escaped literal percent, so it never starts a marker.  A
    /// caller supplies only registry-declared inputs; this helper never treats
    /// an arbitrary percent sequence as tainted.
    #[must_use]
    pub fn occurs_in(self, text: &str) -> bool {
        let marker = self.spelling().as_bytes();
        let bytes = text.as_bytes();
        if marker.len() != 2 || bytes.len() < marker.len() {
            return false;
        }
        let mut index = 0;
        while index + 1 < bytes.len() {
            if bytes[index] != b'%' {
                index += 1;
                continue;
            }
            if bytes[index + 1] == b'%' {
                index += 2;
                continue;
            }
            if bytes[index..].starts_with(marker) {
                return true;
            }
            index += 1;
        }
        false
    }
}

/// Variable frame used by a variable-name option value.
///
/// Tcl commands normally resolve an unqualified variable name in the current
/// call frame. Some APIs instead document an interpreter-global link: Tk's
/// `-textvariable` / `-variable` options are the canonical example. Keeping
/// this on [`OptionArg`] lets SSA and taint analysis normalise the same name
/// without knowing which command or option supplied it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableScope {
    /// Resolve an unqualified name in the invocation's current Tcl frame.
    CurrentFrame,
    /// Resolve an unqualified name from the global namespace.
    Global,
}

/// How many following words a value-taking option consumes.
///
/// No `PartialEq`/`Eq`/`Hash` — `Hook`'s fn-pointer payload has no
/// meaningful equality (two resolvers being pointer-equal isn't "the same
/// option shape," and nothing in this codebase actually compares or
/// hashes an `OptionArity`); see
/// [`CommandSpec`](crate::spec::CommandSpec), which carries several
/// resolver-fn fields and derives neither for the same reason.
#[derive(Debug, Clone, Copy)]
pub enum OptionArity {
    /// Exactly one value (`-index 2`).
    One,
    /// A fixed number of values (`-rect x1 y1 x2 y2` → `Fixed(4)`).
    Fixed(u8),
    /// Word count (and validity) computed from the remaining args — an
    /// arity or a value-content constraint the static shapes above can't
    /// express: "consume everything to `--`/end" (a resolver returning
    /// `args.len() - start`), an option whose span depends on a preceding
    /// flag's value, or a fixed-arity value whose *content* needs
    /// validating (`-errorstack`'s value must be an even-sized list).
    Hook(OptionValueHook),
}

/// A numeric domain an option value may satisfy alongside (or instead of)
/// its literal `values` set — lets a closed enum still accept an
/// arbitrary or ranged integer (`return -code ok|error|...|<int>`)
/// without opening up the whole set to any string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerDomain {
    /// Any Tcl integer literal.
    Any,
    /// Any Tcl integer literal within this inclusive range.
    Range(i64, i64),
    /// A TCP/UDP port (0..=65535). Kept distinct from a plain `Range` so a
    /// hover/inlay-hint pass can try resolving it to a well-known service
    /// name — the lookup table lives downstream of this crate, in
    /// `tcl-bigip`.
    Port,
}

impl IntegerDomain {
    /// Whether `n` satisfies this domain.
    #[must_use]
    pub const fn accepts(self, n: i64) -> bool {
        match self {
            Self::Any => true,
            Self::Range(lo, hi) => lo <= n && n <= hi,
            Self::Port => 0 <= n && n <= 65535,
        }
    }
}

/// What a value-taking option consumes and how to analyse it.
///
/// The `role` mirrors the positional [`ArgRole`], so an option value flows
/// through the *same* analysis passes as a positional argument of that role —
/// body recursion, expr checks, variable flow, channel checks, symbolic-name
/// resolution — instead of being an opaque string.
///
/// No `PartialEq`/`Eq` — carries `arity: OptionArity`, which has none for
/// the same reason.
#[derive(Debug, Clone, Copy)]
pub struct OptionArg {
    /// How many words the option consumes.
    pub arity: OptionArity,
    /// The semantic role of the value word(s).
    pub role: ArgRole,
    /// A second role carried at the same value position, for two-way bindings
    /// (a `-textvariable` name is both written and read by the widget →
    /// `role: VarWrite, also_role: Some(VarRead)`).
    pub also_role: Option<ArgRole>,
    /// Whether this variable-name value is a user-input link whose future
    /// writes originate outside the Tcl program (for example an editable Tk
    /// widget's `-textvariable` or selection `-variable`). Display-only
    /// variable links deliberately leave this false.
    pub taints_var_write: bool,
    /// Frame in which a variable-name value is resolved. Meaningful for
    /// [`ArgRole::VarRead`] / [`ArgRole::VarWrite`]; other roles retain the
    /// [`VariableScope::CurrentFrame`] default.
    pub variable_scope: VariableScope,
    /// When `role` is [`ArgRole::Body`], whether the script runs in the
    /// caller's frame (`Plain`) or a separate definition/dispatch scope
    /// (`Structural` — a Tk `-command` callback).
    pub body_kind: BodyKind,
    /// When an executable option runs. Meaningful when `role` or `also_role`
    /// is [`ArgRole::Body`], [`ArgRole::LambdaLiteral`], or
    /// [`ArgRole::CommandPrefix`]; other roles leave the default.
    pub script_timing: ScriptTiming,
    /// Externally controlled substitutions that the callback host injects
    /// into this stored script immediately before it runs. Empty for normal
    /// scripts and for callbacks carrying only framework metadata.
    pub callback_taint_inputs: &'static [CallbackTaintInput],
    /// Enumerable value set for completion / closed-set checking; empty when
    /// the value is open (an arbitrary string / number / name).
    pub values: &'static [ArgValue],
    /// Whether `values` is exhaustive — a value outside it is an error
    /// (drives the option-aware closed-value check).
    pub closed: bool,
    /// A numeric domain also accepted alongside `values` — lets a closed
    /// enum stay closed while still admitting an integer `values` alone
    /// can't express (`return -code ok|error|...|<int>`). `None` = the
    /// literal set (if any) is the whole story.
    pub integer: Option<IntegerDomain>,
    /// Hint text for the value (e.g. `"channel"`).
    pub hint: &'static str,
    /// When `role` is [`ArgRole::CommandPrefix`], how many arguments the
    /// command appends to the callback when it invokes it — drives the
    /// callback-arity check.  [`AppendedArity::Unknown`] (the default) for
    /// every other role and for prefixes whose count is indeterminate.
    pub appended_arity: AppendedArity,
}

impl OptionArg {
    /// Baseline: single word, generic [`ArgRole::Value`], open set, no hint.
    pub const DEFAULT: Self = Self {
        arity: OptionArity::One,
        role: ArgRole::Value,
        also_role: None,
        taints_var_write: false,
        variable_scope: VariableScope::CurrentFrame,
        body_kind: BodyKind::Plain,
        script_timing: ScriptTiming::SameInvocation,
        callback_taint_inputs: &[],
        values: &[],
        closed: false,
        integer: None,
        hint: "",
        appended_arity: AppendedArity::Unknown,
    };
}

/// What an option consumes: nothing (a boolean flag) or a described value.
///
/// Replaces the old `takes_value: bool` + `value_hint` pair with a single
/// source of truth carrying arity and value role.
///
/// No `PartialEq`/`Eq` — `Takes(OptionArg)` carries one, for the same
/// reason `OptionArg`/`OptionArity` do.
#[derive(Debug, Clone, Copy)]
pub enum OptionValue {
    /// A boolean switch — consumes no following word.
    Flag,
    /// Consumes value word(s) as described.
    Takes(OptionArg),
}

impl OptionValue {
    /// A boolean flag (consumes no value).
    #[must_use]
    pub const fn flag() -> Self {
        Self::Flag
    }

    /// A single generic value with a completion `hint` — the common case,
    /// equivalent to the old `takes_value: true`.
    #[must_use]
    pub const fn value(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A single script-body value evaluated during this invocation, in its own
    /// (Structural) scope (`tcltest::test -body`, for example).
    #[must_use]
    pub const fn script() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Body,
            body_kind: BodyKind::Structural,
            ..OptionArg::DEFAULT
        })
    }

    /// A single script-body value stored for a later callback, in its own
    /// (Structural) scope — a Tk `-command` / `-validatecommand` callback.
    #[must_use]
    pub const fn deferred_script() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Body,
            body_kind: BodyKind::Structural,
            script_timing: ScriptTiming::Deferred,
            ..OptionArg::DEFAULT
        })
    }

    /// A stored script callback with registry-declared external Tk `%`
    /// substitutions.  The list must contain only values whose bytes can be
    /// supplied by the user/event source; framework metadata stays absent.
    #[must_use]
    pub const fn deferred_tainted_script(inputs: &'static [CallbackTaintInput]) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Body,
            body_kind: BodyKind::Structural,
            script_timing: ScriptTiming::Deferred,
            callback_taint_inputs: inputs,
            ..OptionArg::DEFAULT
        })
    }

    /// A single variable-name value read and written by the command
    /// (`-textvariable`, `-variable`).
    #[must_use]
    pub const fn var_name() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::VarWrite,
            also_role: Some(ArgRole::VarRead),
            ..OptionArg::DEFAULT
        })
    }

    /// A two-way variable binding resolved from the global namespace.
    #[must_use]
    pub const fn global_var_name() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::VarWrite,
            also_role: Some(ArgRole::VarRead),
            variable_scope: VariableScope::Global,
            ..OptionArg::DEFAULT
        })
    }

    /// A two-way variable binding whose writes can originate from user input.
    ///
    /// This is intentionally distinct from [`Self::var_name`]: a label or
    /// button `-textvariable` reads application state but is not an input
    /// source, while an entry `-textvariable` is.
    #[must_use]
    pub const fn user_input_var() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::VarWrite,
            also_role: Some(ArgRole::VarRead),
            taints_var_write: true,
            variable_scope: VariableScope::Global,
            ..OptionArg::DEFAULT
        })
    }

    /// A single symbolic-name value — a namespace, proc, method, or class name
    /// (`ArgRole::Name`).  Captures the reference for resolution tooling.
    #[must_use]
    pub const fn name(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Name,
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A single channel-identifier value (`expect -i spawn_id`).
    #[must_use]
    pub const fn channel(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Channel,
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A single value consumed **purely as a boolean** —
    /// [`ArgRole::Boolean`], the first-class registry answer to "is this word
    /// a boolean" (issue #1256).
    ///
    /// The value set is left open on purpose: Tcl accepts every spelling
    /// [`crate::abbrev::boolean_table`] resolves, *including unique prefixes*
    /// (`-blocking tru`), which a closed `values` list cannot express without
    /// making the option-aware closed-value check reject legal code.
    /// Completion and hover read the role and offer
    /// [`crate::abbrev::BOOLEAN_KEYWORDS`] instead.
    ///
    /// Use [`Self::numeric_or_boolean`] for a position that also accepts a
    /// count — `0`/`1` are valid integers, and a rewriting consumer must not
    /// guess which language the author meant.
    #[must_use]
    pub const fn boolean() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Boolean,
            hint: "boolean",
            ..OptionArg::DEFAULT
        })
    }

    /// A single value accepted as **either** a boolean or a number
    /// ([`ArgRole::NumericOrBoolean`]) — declared so a consumer abstains by
    /// construction rather than by inference.
    #[must_use]
    pub const fn numeric_or_boolean(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::NumericOrBoolean,
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A single expression value (argparse `-validate`).
    #[must_use]
    pub const fn expr() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Expr,
            ..OptionArg::DEFAULT
        })
    }

    /// A command-prefix value (`lsort -command cmdPrefix`): the first word is a
    /// command invoked with runtime args appended, not a script body to
    /// recurse (see [`ArgRole::CommandPrefix`]).  The appended count is
    /// [`AppendedArity::Unknown`] (no arity check); use
    /// [`command_prefix_n`](Self::command_prefix_n) when it is known.
    #[must_use]
    pub const fn command_prefix(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::CommandPrefix,
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A command-prefix value stored for later invocation when the appended
    /// arity is not known.
    #[must_use]
    pub const fn deferred_command_prefix(hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::CommandPrefix,
            hint,
            script_timing: ScriptTiming::Deferred,
            ..OptionArg::DEFAULT
        })
    }

    /// A command-prefix value whose invoked-arity is known — `lsort -command`
    /// appends 2 (`command_prefix_n("cmdPrefix", AppendedArity::Exactly(2))`),
    /// `-xscrollcommand` appends 2, `socket -server` appends 3.  Drives the
    /// callback-arity check against the referenced proc.
    #[must_use]
    pub const fn command_prefix_n(hint: &'static str, appended: AppendedArity) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::CommandPrefix,
            hint,
            appended_arity: appended,
            ..OptionArg::DEFAULT
        })
    }

    /// A command-prefix value stored for later invocation, with a known
    /// appended arity. Tk's `text sync -command` appends no arguments but
    /// schedules the prefix after line metrics become current.
    #[must_use]
    pub const fn deferred_command_prefix_n(hint: &'static str, appended: AppendedArity) -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::CommandPrefix,
            hint,
            appended_arity: appended,
            script_timing: ScriptTiming::Deferred,
            ..OptionArg::DEFAULT
        })
    }

    /// A single value drawn from an enumerable set.  `closed` marks the set as
    /// exhaustive (a value outside it is flagged).
    #[must_use]
    pub const fn enumerated(values: &'static [ArgValue], closed: bool, hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            values,
            closed,
            hint,
            ..OptionArg::DEFAULT
        })
    }

    /// A fixed number of value words of the given `role`
    /// (`-rect x1 y1 x2 y2` → `fixed(4, ArgRole::Value, "coord")`).
    #[must_use]
    pub const fn fixed(n: u8, role: ArgRole, hint: &'static str) -> Self {
        Self::Takes(OptionArg {
            arity: OptionArity::Fixed(n),
            role,
            hint,
            ..OptionArg::DEFAULT
        })
    }
}

/// Metadata for a switch-like option (`-nonewline`, `-nocase`, etc.).
///
/// No `PartialEq`/`Eq` — carries `value: OptionValue`, which has none, for
/// the same reason `CommandSpec` (also fn-pointer-hook-bearing) derives
/// neither.
#[derive(Debug, Clone)]
pub struct OptionSpec {
    /// Option name (e.g. `"-nonewline"`).
    pub name: &'static str,
    /// What the option consumes — a boolean flag or a described value.
    /// Replaces the old `takes_value` / `value_hint` pair.
    pub value: OptionValue,
    /// Short description.
    pub detail: &'static str,
    /// Dialect membership.  `None` means "inherit from the parent
    /// `CommandSpec` / `SubCommand` dialects" — the common case.
    /// Set this to restrict an option added in a specific Tcl
    /// version (e.g. `lsearch -stride` is Tcl 8.6+, `clock scan
    /// -validate` is Tcl 9.0+) so the option doesn't surface in
    /// older dialects.
    pub surface: Option<&'static [SpecSurface]>,
    /// Documented alternate spellings Tcl accepts for this same option
    /// (e.g. `-bd` for `-borderwidth`, `-bg` for `-background`).  These are
    /// *explicit* aliases the command's own option table recognises — not the
    /// general unambiguous-prefix matching Tcl also allows.  Validation,
    /// value-arity, and option lookup treat an alias exactly like `name`;
    /// completion offers only the canonical `name`.
    pub aliases: &'static [&'static str],
    /// Introduction / deprecation / retirement releases of this option on its
    /// owning *package*'s version axis (e.g. `entry -placeholder` is
    /// introduced in Tk `8.7`).  [`Lifecycle::UNSPECIFIED`] means "present in
    /// every version of the owning package".  Gated against the version
    /// resolved from `package require` — orthogonal to `dialects` (which
    /// gates on the Tcl *core* version).
    pub lifecycle: Lifecycle,
    /// Documented minimum abbreviation length for this option, when the
    /// command promises a longer minimum than uniqueness alone requires.
    /// `None` (the norm) means Tcl's `Tcl_GetIndexFromObj` rule applies:
    /// **any** unique prefix resolves (`lsearch -noc` ⇒ `-nocase`).  See
    /// [`crate::abbrev`].
    pub min_abbrev: Option<u8>,
}

impl OptionSpec {
    /// Default value for all fields — used with `..OptionSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        value: OptionValue::Flag,
        detail: "",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    };

    /// This option as a [`Keyword`] for abbreviation resolution.
    #[must_use]
    pub const fn as_keyword(&self) -> Keyword<'static> {
        Keyword {
            name: self.name,
            min_abbrev: self.min_abbrev,
        }
    }

    /// Whether this option consumes a following value (any arity ≥ 1).
    #[must_use]
    pub const fn takes_value(&self) -> bool {
        !matches!(self.value, OptionValue::Flag)
    }

    /// Completion hint for the option's value (empty for a flag).
    #[must_use]
    pub const fn value_hint(&self) -> &'static str {
        match self.value {
            OptionValue::Flag => "",
            OptionValue::Takes(arg) => arg.hint,
        }
    }

    /// The primary [`ArgRole`] of the option's value, if it takes one.
    #[must_use]
    pub const fn value_role(&self) -> Option<ArgRole> {
        match self.value {
            OptionValue::Flag => None,
            OptionValue::Takes(arg) => Some(arg.role),
        }
    }

    /// Whether the option's value is consumed **purely as a boolean**, so
    /// every accepted spelling of the same truth value is interchangeable.
    ///
    /// Reads the declared [`ArgRole::Boolean`] fact (issue #1256) — never
    /// inferred from the value set, which only ever covered the handful of
    /// options that happened to enumerate `true`/`false` and missed every
    /// option declared with an open value or a bare hint.
    /// [`ArgRole::NumericOrBoolean`] answers `false`: `0`/`1` are valid
    /// integers there too.
    #[must_use]
    pub const fn value_is_boolean(&self) -> bool {
        match self.value {
            OptionValue::Flag => false,
            OptionValue::Takes(arg) => arg.role.consumes_boolean(),
        }
    }

    /// A secondary role carried at the value position (two-way var-name).
    #[must_use]
    pub const fn value_also_role(&self) -> Option<ArgRole> {
        match self.value {
            OptionValue::Flag => None,
            OptionValue::Takes(arg) => arg.also_role,
        }
    }

    /// Whether the option links a variable to an external user-input source.
    #[must_use]
    pub const fn taints_var_write(&self) -> bool {
        matches!(self.value, OptionValue::Takes(arg) if arg.taints_var_write)
    }

    /// Variable frame declared by this option value.
    #[must_use]
    pub const fn value_variable_scope(&self) -> Option<VariableScope> {
        match self.value {
            OptionValue::Takes(arg)
                if matches!(arg.role, ArgRole::VarRead | ArgRole::VarWrite)
                    || matches!(arg.also_role, Some(ArgRole::VarRead | ArgRole::VarWrite)) =>
            {
                Some(arg.variable_scope)
            }
            OptionValue::Flag | OptionValue::Takes(_) => None,
        }
    }

    /// Timing declared by a script or command-prefix option, or `None` for a
    /// flag or a non-executable value.
    #[must_use]
    pub const fn value_script_timing(&self) -> Option<ScriptTiming> {
        match self.value {
            OptionValue::Takes(arg)
                if arg.role.has_script_timing()
                    || matches!(arg.also_role, Some(role) if role.has_script_timing()) =>
            {
                Some(arg.script_timing)
            }
            OptionValue::Flag | OptionValue::Takes(_) => None,
        }
    }

    /// Externally controlled callback substitutions declared for this option.
    #[must_use]
    pub const fn value_callback_taint_inputs(&self) -> &'static [CallbackTaintInput] {
        match self.value {
            OptionValue::Flag => &[],
            OptionValue::Takes(arg) => arg.callback_taint_inputs,
        }
    }

    /// The appended-arity of a [`ArgRole::CommandPrefix`] value
    /// ([`AppendedArity::Unknown`] for a flag or any other role).
    #[must_use]
    pub const fn value_appended_arity(&self) -> AppendedArity {
        match self.value {
            OptionValue::Flag => AppendedArity::Unknown,
            OptionValue::Takes(arg) => arg.appended_arity,
        }
    }

    /// The option value's enumerable set (empty when open / a flag).
    #[must_use]
    pub const fn value_values(&self) -> &'static [ArgValue] {
        match self.value {
            OptionValue::Flag => &[],
            OptionValue::Takes(arg) => arg.values,
        }
    }

    /// Whether the option value's set is closed (exhaustive).
    #[must_use]
    pub const fn value_is_closed(&self) -> bool {
        matches!(self.value, OptionValue::Takes(arg) if arg.closed)
    }

    /// The option value's integer domain, if it has one (`None` for a
    /// flag, an open string, or an enumerated value with no numeric
    /// alternative).
    #[must_use]
    pub const fn value_integer_domain(&self) -> Option<IntegerDomain> {
        match self.value {
            OptionValue::Flag => None,
            OptionValue::Takes(arg) => arg.integer,
        }
    }

    /// The option value's dynamic arity/content-validation hook, if its
    /// arity is [`OptionArity::Hook`] rather than a static shape.
    #[must_use]
    pub const fn value_arity_hook(&self) -> Option<OptionValueHook> {
        match self.value {
            OptionValue::Flag => None,
            OptionValue::Takes(arg) => match arg.arity {
                OptionArity::Hook(f) => Some(f),
                OptionArity::One | OptionArity::Fixed(_) => None,
            },
        }
    }

    /// The half-open `args` range this option consumes as its value(s) when the
    /// option word sits at `flag_idx`.
    ///
    /// Honours arity ([`OptionArity`]), clamps to the argument list, and stops
    /// at an option terminator `--` (so `-index --` consumes nothing, matching
    /// the existing value-colouring convention).  Empty for a
    /// [`OptionValue::Flag`].  The single source of the value-span logic shared
    /// by [`Self::value_indices`] / [`Self::value_word_count`] and, through
    /// them, every option-scanning loop.
    ///
    /// The `--` scan is bounded to the consumed window (`start + want`) rather
    /// than the whole remaining argument list, so a `One`/`Fixed` option is
    /// O(arity) not O(remaining args) — the option-scan loops call this once
    /// per value-taking flag, so an unbounded scan would be quadratic in the
    /// number of such flags on one command.
    fn value_span<S: AsRef<str>>(&self, args: &[S], flag_idx: usize) -> core::ops::Range<usize> {
        let OptionValue::Takes(arg) = self.value else {
            return 0..0;
        };
        let hard_end = args.len();
        let start = (flag_idx + 1).min(hard_end);
        let want = match arg.arity {
            OptionArity::One => 1,
            OptionArity::Fixed(n) => usize::from(n),
            OptionArity::Hook(resolve) => {
                let owned: Vec<&str> = args.iter().map(S::as_ref).collect();
                resolve(&owned, start).words
            }
        };
        let window_end = (start + want).min(hard_end);
        // Only a `--` inside the consumed window matters; bound the scan to it.
        let term = args[start..window_end]
            .iter()
            .position(|w| w.as_ref() == "--")
            .map_or(window_end, |p| start + p);
        start..term
    }

    /// The absolute indices into `args` this option consumes as its value(s)
    /// when the option word sits at `flag_idx` (see [`Self::value_span`]).
    #[must_use]
    pub fn value_indices<S: AsRef<str>>(&self, args: &[S], flag_idx: usize) -> Vec<usize> {
        self.value_span(args, flag_idx).collect()
    }

    /// How many value words this option consumes at `flag_idx`
    /// (see [`Self::value_span`]).  Does not allocate.
    #[must_use]
    pub fn value_word_count<S: AsRef<str>>(&self, args: &[S], flag_idx: usize) -> usize {
        self.value_span(args, flag_idx).len()
    }

    /// Check whether this option is available in *dialect*.
    ///
    /// If the option has its own `dialects` set, use it.  Otherwise
    /// inherit from *`parent_surface`* (the parent `CommandSpec` or
    /// `SubCommand`).  When either side is `None`, the option is
    /// considered available (no restriction).
    #[must_use]
    pub fn supports_dialect(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        parent_surface: Option<&'static [SpecSurface]>,
    ) -> bool {
        let Some(rows) = self.surface.or(parent_surface) else {
            return true;
        };
        surface_admits(rows, dialect.as_ref())
    }

    /// Whether `option_name` is this option's canonical name or an alias.
    #[must_use]
    pub fn matches(&self, option_name: &str) -> bool {
        self.name == option_name || self.aliases.contains(&option_name)
    }

    /// Whether this option exists given the resolved *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor derived from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` (no version constraint known) is permissive; an option with an
    /// unspecified lifecycle is always available.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This option's lifecycle state at the resolved *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

/// Return the index of the first positional argument after leading options.
///
/// The scan starts at `scan_start`, skips each option word and the value words
/// declared by its [`OptionSpec`], and stops after the `--` terminator. Unknown
/// option-looking words are treated as flag-shaped words and consume no value;
/// this preserves the analyser's recovery behaviour for malformed calls while
/// keeping all known value arities in the registry.
#[must_use]
pub fn first_positional_index<S: AsRef<str>>(
    options: &[OptionSpec],
    args: &[S],
    scan_start: usize,
) -> usize {
    let mut index = scan_start.min(args.len());
    while let Some(word) = args.get(index).map(AsRef::as_ref) {
        if word == "--" {
            return index + 1;
        }
        if !word.starts_with('-') {
            break;
        }
        let consumed = options
            .iter()
            .find(|option| option.matches(word))
            .map_or(0, |option| option.value_word_count(args, index));
        index = index.saturating_add(1 + consumed);
    }
    index
}

/// Completion / hover metadata for a single enumerable
/// positional-argument value.
///
/// Used for arguments
/// whose value comes from a fixed set — e.g. the character
/// class in `string is <class>`, the event name in iRules
/// `when <EVENT>`, or a subcommand keyword.  The completion
/// provider surfaces `value` (with `detail` as the right-hand
/// description) when the cursor sits on the matching argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgValue {
    /// The literal value (e.g. `"alnum"`).
    pub value: &'static str,
    /// Short description for the completion list.
    pub detail: &'static str,
    /// The lowest **Tcl core** release that accepts this value, or `None` for
    /// every release — the argument-DSL rung of the granularity ladder
    /// (dialect-profile-model.md §6: `string is dict` raises before 9.0
    /// even though the `is` subcommand itself is universal). Checked
    /// against the profile's `effective_tcl_version`, and reported by W137.
    ///
    /// This is the Tcl-core axis. The owning-package axis is
    /// [`Self::lifecycle`]; the two are orthogonal and a value may carry
    /// either, both, or neither.
    pub min_tcl: Option<tcl_dialect::TclVersion>,
    /// Introduction / deprecation / retirement releases of this value on the
    /// owning *package*'s version axis — the same axis every other registry
    /// entity's [`Lifecycle`] sits on, resolved from `package require` (or an
    /// ambient profile pin) and reported by W135 / W139 / W144.
    /// [`Lifecycle::UNSPECIFIED`] means "present in every version of the
    /// owning package".
    ///
    /// Distinct from [`Self::min_tcl`], which is the Tcl-core-version floor.
    pub lifecycle: Lifecycle,
    /// Canonical integer equivalent, when this value has one (`"ok"` →
    /// `Some(0)`). `None` for a plain enum member with no numeric
    /// pairing — every pre-existing `ArgValue` literal, unchanged in
    /// meaning.
    pub code: Option<i64>,
}

impl ArgValue {
    /// Default value for all fields — used with `..ArgValue::DEFAULT`.
    pub const DEFAULT: Self = Self {
        value: "",
        detail: "",
        min_tcl: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        code: None,
    };

    /// Whether this value exists given the resolved *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` is permissive. This tests the owning-package axis only — the
    /// Tcl-core floor is [`Self::min_tcl`].
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This value's lifecycle state at the resolved *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

/// Classification of a command invocation form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormKind {
    /// Default form.
    Default,
    /// Getter form (read-only).
    Getter,
    /// Setter form (modifying).
    Setter,
}

/// A concrete invocation form of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSpec {
    /// Form classification.
    pub kind: FormKind,
    /// Human-readable invocation signature.
    pub synopsis: &'static str,
    /// Dialects in which this form applies, when narrower than the
    /// command's own availability — e.g. `return`'s bare `"return"`
    /// synopsis only documents an iRules event-body form, even though
    /// `return` itself is universal Tcl (`CommandSpec::surface: None`).
    /// `None` = inherits the command's own dialect gating, so every form
    /// declared before this field existed keeps its meaning unchanged.
    /// Mirrors [`crate::forms::CommandForm::dialects`].
    pub surface: Option<&'static [SpecSurface]>,
    /// Introduction / deprecation / retirement releases of this invocation
    /// form on the owning command's package version axis — a synopsis a later
    /// release added or withdrew. [`Lifecycle::UNSPECIFIED`] means the form is
    /// documented in every package version; orthogonal to [`Self::dialects`],
    /// which gates on the Tcl *core* version.
    pub lifecycle: Lifecycle,
}

impl FormSpec {
    /// Baseline: [`FormKind::Default`], empty synopsis, no dialect
    /// restriction, no lifecycle — used with `..FormSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        kind: FormKind::Default,
        synopsis: "",
        surface: None,
        lifecycle: Lifecycle::UNSPECIFIED,
    };

    /// Whether this form is documented given the resolved
    /// *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` is permissive.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This form's lifecycle state at the resolved *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_dialect_inherits_from_parent_when_unset() {
        let opt = OptionSpec {
            name: "-foo",
            value: OptionValue::flag(),
            detail: "",
            surface: None,
            aliases: &[],
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
        };
        // No parent: always available.
        assert!(opt.supports_dialect(Some(SpecSurface::TCL84), None));
        // Parent allows everything: available.
        assert!(opt.supports_dialect(Some(SpecSurface::TCL84), Some(SpecSurface::ALL_TCL)));
        // Parent restricts: inherit the restriction.
        assert!(opt.supports_dialect(Some(SpecSurface::TCL86), Some(SpecSurface::TCL86_PLUS)));
        assert!(!opt.supports_dialect(Some(SpecSurface::TCL85), Some(SpecSurface::TCL86_PLUS)));
    }

    #[test]
    fn supports_dialect_own_set_overrides_parent() {
        // `lsearch -stride` is Tcl 8.6+ even though `lsearch` itself
        // is available since 8.4.  The option's own dialects field
        // wins.
        let opt = OptionSpec {
            name: "-stride",
            value: OptionValue::value("int"),
            detail: "",
            surface: Some(SpecSurface::TCL86_PLUS),
            aliases: &[],
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
        };
        assert!(opt.supports_dialect(Some(SpecSurface::TCL86), Some(SpecSurface::ALL_TCL)));
        assert!(opt.supports_dialect(Some(SpecSurface::TCL90), Some(SpecSurface::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(SpecSurface::TCL84), Some(SpecSurface::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(SpecSurface::TCL85), Some(SpecSurface::ALL_TCL)));
    }

    #[test]
    fn supports_dialect_none_active_is_unrestricted() {
        // No active dialect = treat option as available (e.g.
        // unscoped completion).
        let opt = OptionSpec {
            name: "-x",
            value: OptionValue::flag(),
            detail: "",
            surface: Some(SpecSurface::TCL90),
            aliases: &[],
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
        };
        assert!(opt.supports_dialect(None, Some(SpecSurface::TCL90)));
    }

    #[test]
    fn first_positional_index_consumes_declared_values_and_terminator() {
        let options = [
            OptionSpec {
                name: "-flag",
                ..OptionSpec::DEFAULT
            },
            OptionSpec {
                name: "-value",
                value: OptionValue::value("word"),
                ..OptionSpec::DEFAULT
            },
        ];
        assert_eq!(
            first_positional_index(&options, &["-flag", "-value", "v", "subject"], 0),
            3
        );
        assert_eq!(
            first_positional_index(&options, &["-value", "v", "--", "-subject"], 0),
            3
        );
        // Unknown option-looking words remain one-word recovery skips.
        assert_eq!(
            first_positional_index(&options, &["-unknown", "subject"], 0),
            1
        );
    }
}
