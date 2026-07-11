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

use crate::arg_role::{AppendedArity, ArgRole};
use crate::body_kind::BodyKind;
use crate::dialects::DialectSet;

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

/// How many following words a value-taking option consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionArity {
    /// Exactly one value (`-index 2`).
    One,
    /// A fixed number of values (`-rect x1 y1 x2 y2` → `Fixed(4)`).
    Fixed(u8),
    /// All following words up to the next `--` / end of the argument list.
    Rest,
}

/// What a value-taking option consumes and how to analyse it.
///
/// The `role` mirrors the positional [`ArgRole`], so an option value flows
/// through the *same* analysis passes as a positional argument of that role —
/// body recursion, expr checks, variable flow, channel checks, symbolic-name
/// resolution — instead of being an opaque string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionArg {
    /// How many words the option consumes.
    pub arity: OptionArity,
    /// The semantic role of the value word(s).
    pub role: ArgRole,
    /// A second role carried at the same value position, for two-way bindings
    /// (a `-textvariable` name is both written and read by the widget →
    /// `role: VarWrite, also_role: Some(VarRead)`).
    pub also_role: Option<ArgRole>,
    /// When `role` is [`ArgRole::Body`], whether the script runs in the
    /// caller's frame (`Plain`) or a separate definition/dispatch scope
    /// (`Structural` — a Tk `-command` callback).
    pub body_kind: BodyKind,
    /// Enumerable value set for completion / closed-set checking; empty when
    /// the value is open (an arbitrary string / number / name).
    pub values: &'static [ArgValue],
    /// Whether `values` is exhaustive — a value outside it is an error
    /// (drives the option-aware closed-value check).
    pub closed: bool,
    /// Hint text for the value (e.g. `"channel"`), migrated from the old
    /// `value_hint` field.
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
        body_kind: BodyKind::Plain,
        values: &[],
        closed: false,
        hint: "",
        appended_arity: AppendedArity::Unknown,
    };
}

/// What an option consumes: nothing (a boolean flag) or a described value.
///
/// Replaces the old `takes_value: bool` + `value_hint` pair with a single
/// source of truth carrying arity and value role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// A single script-body value run in its own (Structural) scope — a Tk
    /// `-command` / `-validatecommand` callback.
    #[must_use]
    pub const fn script() -> Self {
        Self::Takes(OptionArg {
            role: ArgRole::Body,
            body_kind: BodyKind::Structural,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub dialects: Option<DialectSet>,
    /// Documented alternate spellings Tcl accepts for this same option
    /// (e.g. `-bd` for `-borderwidth`, `-bg` for `-background`).  These are
    /// *explicit* aliases the command's own option table recognises — not the
    /// general unambiguous-prefix matching Tcl also allows.  Validation,
    /// value-arity, and option lookup treat an alias exactly like `name`;
    /// completion offers only the canonical `name`.
    pub aliases: &'static [&'static str],
    /// Minimum *package* version that introduced this option, as a dotted Tcl
    /// version string (e.g. `entry -placeholder` needs Tk `8.7`).  `None`
    /// means "present in every version of the owning package".  Gated against
    /// the version resolved from `package require` — orthogonal to `dialects`
    /// (which gates on the Tcl *core* version).
    pub min_version: Option<&'static str>,
}

impl OptionSpec {
    /// Default value for all fields — used with `..OptionSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        value: OptionValue::Flag,
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    };

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

    /// A secondary role carried at the value position (two-way var-name).
    #[must_use]
    pub const fn value_also_role(&self) -> Option<ArgRole> {
        match self.value {
            OptionValue::Flag => None,
            OptionValue::Takes(arg) => arg.also_role,
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
            OptionArity::Rest => hard_end - start,
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
    /// inherit from *`parent_dialects`* (the parent `CommandSpec` or
    /// `SubCommand`).  When either side is `None`, the option is
    /// considered available (no restriction).
    #[must_use]
    pub fn supports_dialect(
        &self,
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> bool {
        let Some(active) = dialect else {
            return true;
        };
        if let Some(own) = self.dialects {
            return own.contains(active);
        }
        let Some(parent) = parent_dialects else {
            return true;
        };
        parent.contains(active)
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
    /// `None` (no version constraint known) is permissive; an option with no
    /// `min_version` is always available.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        match (self.min_version, package_version) {
            (Some(min), Some(have)) => crate::version::meets_min(have, min),
            _ => true,
        }
    }
}

/// Index of the first positional argument in `args`, scanning from `scan_start`.
///
/// Skips the leading option words of a command whose options are `options`:
///
/// * a `--` terminator is consumed and scanning stops (the next word is the
///   first positional);
/// * a recognised option (canonical name or alias, via [`OptionSpec::matches`])
///   is skipped along with the value word(s) it consumes at that position —
///   arity-aware via [`OptionSpec::value_word_count`], so a value that itself
///   looks like a flag (`-start -3`) is not re-scanned;
/// * an unrecognised `-word` is treated as a value-less flag (skip one word),
///   matching the leading-option-skip convention shared by the analyser and
///   semantic-token loops;
/// * the first word that does not start with `-` ends the scan.
///
/// This is the single source of the "where do a command's leading options end"
/// logic — every `arg_role_resolver` and pattern-position loop routes through
/// it rather than re-hardcoding which options take a value. Returns `args.len()`
/// when every word is an option (no positional present).
#[must_use]
pub fn first_positional_index<S: AsRef<str>>(
    options: &[OptionSpec],
    args: &[S],
    scan_start: usize,
) -> usize {
    let mut i = scan_start;
    while i < args.len() {
        let word = args[i].as_ref();
        if !word.starts_with('-') {
            break;
        }
        if word == "--" {
            i += 1;
            break;
        }
        let value_words = options
            .iter()
            .find(|o| o.matches(word))
            .map_or(0, |o| o.value_word_count(args, i));
        i += 1 + value_words;
    }
    i
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
            dialects: None,
            aliases: &[],
            min_version: None,
        };
        // No parent: always available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), None));
        // Parent allows everything: available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        // Parent restricts: inherit the restriction.
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::TCL86_PLUS)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::TCL86_PLUS)));
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
            dialects: Some(DialectSet::TCL86_PLUS),
            aliases: &[],
            min_version: None,
        };
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::ALL_TCL)));
        assert!(opt.supports_dialect(Some(DialectSet::TCL90), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::ALL_TCL)));
    }

    #[test]
    fn supports_dialect_none_active_is_unrestricted() {
        // No active dialect = treat option as available (e.g.
        // unscoped completion).
        let opt = OptionSpec {
            name: "-x",
            value: OptionValue::flag(),
            detail: "",
            dialects: Some(DialectSet::TCL90),
            aliases: &[],
            min_version: None,
        };
        assert!(opt.supports_dialect(None, Some(DialectSet::TCL90)));
    }

    /// A `regexp`/`regsub`-shaped option table: `-start` takes an index value,
    /// the rest are boolean flags, and `--` terminates.
    const REGEXP_LIKE: &[OptionSpec] = &[
        OptionSpec {
            name: "-nocase",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-all",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-start",
            value: OptionValue::value("index"),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "--",
            ..OptionSpec::DEFAULT
        },
    ];

    #[test]
    fn first_positional_index_skips_flags_and_value_words() {
        // No options → arg 0 is the first positional.
        assert_eq!(first_positional_index(REGEXP_LIKE, &["exp", "str"], 0), 0);
        // Boolean flags skip one word each.
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-nocase", "-all", "exp", "str"], 0),
            2
        );
        // `-start` consumes its value word — the value is *not* counted as the
        // pattern (the ISSUE_022 regression: name-only skips got this wrong).
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-start", "3", "exp", "str"], 0),
            2
        );
        // Mixed flags plus a value-taking option.
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-nocase", "-start", "3", "exp"], 0),
            3
        );
    }

    #[test]
    fn first_positional_index_handles_terminator_and_edges() {
        // `--` is consumed; the next word is the first positional even if it
        // looks like a flag.
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-nocase", "--", "-weird", "str"], 0),
            2
        );
        // A `-start`-looking value after `--` is a positional, not an option.
        assert_eq!(first_positional_index(REGEXP_LIKE, &["--", "-start"], 0), 1);
        // Unrecognised `-word` is treated as a value-less flag.
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-mystery", "exp"], 0),
            1
        );
        // Every word an option → returns the length (no positional).
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["-nocase", "-all"], 0),
            2
        );
        // `scan_start` skips a leading subcommand word.
        assert_eq!(
            first_positional_index(REGEXP_LIKE, &["sub", "-nocase", "exp"], 1),
            2
        );
    }
}
