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

//! `regexp` — match a regular expression against a string.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

// Confirmed byte-for-byte stable synopsis/switch text across the fetched
// Tcl 8.4, 8.5, 8.6, 9.0, and 9.1 manpages: no switch was added, removed,
// or renamed for `regexp` anywhere in that range (contrast `regsub`,
// whose `-command` is 9.0+ only). Both forms below are therefore
// universal Tcl, not dialect- or version-specific: the second documents
// `-about`'s reduced arity (see the `arity:` comment in `spec()` below)
// as its own invocation shape, the same way `open`'s pipe form gets its
// own `FormSpec` entry.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "regexp ?switches? exp string ?matchVar? ?subMatchVar subMatchVar ...?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "regexp -about ?switches? exp",
        ..FormSpec::DEFAULT
    },
];

/// `regexp ?switches? exp string ?matchVar ...?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), arg 0 is the
/// pattern, arg 1 the string, and args 2+ are capture variables.  Resolve
/// `VarWrite` for every trailing capture var dynamically (the leading-option
/// shift means a static slot list cannot place them).
///
/// Two switches change that layout, so the table reads them rather than
/// merely skipping them. Both rules are the ones
/// [`tcl_cmd_core`'s `regexp`](../../../../tcl-cmd-core/src/regex.rs)
/// implements from the C source, and both were measured identical on tclsh
/// 8.4.20, 8.6.18 and 9.0.4:
///
/// * **`-about`** describes `exp` and returns before it ever looks at a
///   subject, silently ignoring every later word — `regexp -about {(a)}
///   extraarg` is `1 {}` and leaves `extraarg` untouched. Nothing after the
///   pattern carries a role.
/// * **`-inline`** returns the match data as a list, and any trailing word is
///   `regexp match variables not allowed when using -inline` on every
///   release. That is an arity finding, never a write.
///
/// Attributing `VarWrite` regardless deleted a live store: tclsh prints `old`
/// for `set v old; regexp -about {a(b)} somestring v; puts $v`, and the
/// optimised program failed outright with `can't read "v"` (#2135).
///
/// `--` is handled by the scan, not here: `regexp -- -about $s v` reports no
/// switches, so `-about` is the pattern and `v` is a match variable — which
/// is what tclsh does.
fn regexp_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let i = first_positional_index(REGEXP_OPTIONS, args, 0);
    let pattern = std::iter::once((i, ArgRole::Pattern));
    // The two switches are option rows declaring their layout effect, read
    // through the generic walk's shifts rather than by spelling: `-inline`
    // suppresses the match-variable writes, and `-about` shrinks the
    // reservation after the options from `exp string` to `exp` alone — with
    // no subject there is no match data, so nothing after it is a variable.
    let effects = crate::option_effect::option_effects_with(
        REGEXP_OPTIONS,
        FAMILIES,
        InvocationArguments::literals(args),
        0,
        None,
        PrefixMatching::Strict,
    );
    let reserved = effects
        .reserved_trailing_words()
        .map_or(SUBJECT_LAYOUT, usize::from);
    let layout_has_no_match_vars =
        effects.suppresses(ArgRole::VarWrite) || reserved < SUBJECT_LAYOUT;
    if layout_has_no_match_vars {
        return pattern
            .filter_map(|(index, role)| u8::try_from(index).ok().map(|index| (index, role)))
            .collect();
    }
    let capture_start = i + reserved; // skip pattern + string
    pattern
        .chain((capture_start..args.len()).map(|index| (index, ArgRole::VarWrite)))
        .filter_map(|(index, role)| u8::try_from(index).ok().map(|index| (index, role)))
        .collect()
}

/// The operands the layout reserves after the switches when it matches:
/// `exp string`.
const SUBJECT_LAYOUT: usize = 2;

/// The family of the switches that reshape the operand layout rather than
/// move an axis.
const LAYOUT: &str = "layout";

const FAMILIES: &[OptionEffectFamily] = &[OptionEffectFamily {
    name: LAYOUT,
    base: FamilyBase::AllOn,
    combine: FamilyCombine::Accumulate,
    surface: None,
}];

/// tclsh 8.4–9.1: `regexp -inline {a(b)} ab v` → this error. A match
/// variable after `-inline` is a finding (W147), never a write.
const INLINE_FORBIDS_MATCH_VARIABLES: OptionRelation = OptionRelation {
    message: Some("regexp match variables not allowed when using -inline"),
    ..Relation::forbids(OptionTerm::Option("-inline"), &[OptionTerm::Argument(2)])
};

/// A switch whose presence reshapes the operand layout.
const fn layout_flag(
    name: &'static str,
    detail: &'static str,
    kind: OptionEffectKind,
) -> OptionSpec {
    OptionSpec {
        effect: Some(OptionEffect {
            kind,
            family: LAYOUT,
        }),
        ..flag(name, detail)
    }
}

/// A boolean switch (`-flag`) — takes no value, available in all dialects.
const fn flag(name: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::flag(),
        detail,
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
        effect: None,
    }
}

/// The 11 `regexp` switches — confirmed byte-for-byte stable (same 11
/// names, same value-taking shape) across the fetched Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1 manpages, so none carries a `surface:` restriction.
/// `-start` is the only switch that takes a value (an `index`); the rest
/// are boolean flags.  `--` terminates option parsing.
///
/// `-start`'s *value grammar* is a genuine behavioural change, not merely
/// added prose: the "the index value is interpreted in the same manner as
/// the index argument to string index" sentence appears in the manpage
/// only from 8.5 onward, and `generic/tclCmdMZ.c`'s `Tcl_RegexpObjCmd`
/// confirms the C level matches — Tcl 8.4 (`core-8-4-20`) parses the
/// value with plain `Tcl_GetIntFromObj` (a literal integer only, clamped
/// to >= 0; `end`/`end-N` fails with "expected integer"), while 8.5
/// (`core-8-5-19`) through 9.1 (`main`, i.e. `9.1b0`) parse it with
/// `TclGetIntForIndexM`, the same `end`/`end-N` grammar `string index`
/// uses. See the `-start` `OptionSpec`'s `detail` text below.
const REGEXP_OPTIONS: &[OptionSpec] = &[
    flag(
        "-nocase",
        "Match case-insensitively: upper-case characters in string are treated as lower case.",
    ),
    flag(
        "-expanded",
        "Ignore whitespace and # comments in exp (the (?x) embedded flag).",
    ),
    flag(
        "-line",
        "Enable newline-sensitive matching: equivalent to giving both -linestop and -lineanchor together (the (?n) embedded flag).",
    ),
    flag(
        "-linestop",
        "Make . and [^...] bracket expressions stop at a newline instead of matching through it (the (?p) embedded flag).",
    ),
    flag(
        "-lineanchor",
        "Make ^ and $ also match immediately after/before a newline, not just the start/end of the whole string (the (?w) embedded flag).",
    ),
    flag(
        "-all",
        "Match as many times as possible, returning the total match count instead of 1/0; with match variables given, they end up holding the last match only.",
    ),
    layout_flag(
        "-inline",
        "Return the match data as a list instead of writing match variables (illegal to combine with a matchVar/subMatchVar argument). With -all, every match's data is concatenated into one flat list.",
        OptionEffectKind::SuppressesRole(ArgRole::VarWrite),
    ),
    flag(
        "-indices",
        "Store each matchVar/subMatchVar as a {first last} character-index pair into string instead of the matched text.",
    ),
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "Character index into string to start matching at. Tcl 8.4 accepts only a plain non-negative integer; Tcl 8.5 and later accept the fuller index syntax used by string index (e.g. end, end-N). ^ no longer anchors to the string's real start there, though \\A still does; -indices results stay relative to the whole string, and index is clamped to the string's bounds.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
        effect: None,
    },
    layout_flag(
        "-about",
        "Skip matching and instead return {subexpressionCount propertyList} describing the compiled pattern, for debugging; needs only exp — string may be omitted.",
        OptionEffectKind::ReservesTrailingWords(1),
    ),
    layout_flag(
        "--",
        "Ends switch parsing; the next word is treated as exp even if it begins with -.",
        OptionEffectKind::EndsOptions,
    ),
];

/// Hover documentation for `regexp`.
const REGEXP_HOVER: HoverSnippet = HoverSnippet {
    summary: "Match a regular expression against a string.",
    synopsis: &[
        "regexp ?switches? exp string ?matchVar? ?subMatchVar subMatchVar ...?",
        "regexp -about ?switches? exp",
    ],
    snippet: "Determines whether exp (an ARE — see re_syntax) matches part or all of string, returning 1 on a match and 0 otherwise. Extra arguments after string name variables to receive match data: matchVar gets the overall matched text, and each subMatchVar gets the text of one parenthesized subexpression in exp, left to right; a subMatchVar with no corresponding subexpression, or whose subexpression did not participate in the match, is set to the empty string (or to \"-1 -1\" when -indices is given).\n\n-all repeats the match as many times as possible and returns the total match count instead of 1/0; if match variables are also given, they end up holding only the last match. -inline returns the match data as a list instead of writing match variables — combining it with any matchVar/subMatchVar argument is an error — and combined with -all the per-match lists are concatenated into one flat list (the whole match plus one element per subexpression, for each match). -about skips matching entirely and returns {subexpressionCount propertyList} describing the compiled pattern; it needs only exp, string may be omitted. -start index begins the search index characters into string without anchoring ^ there (\\A still anchors to index), and any -indices results stay relative to the start of the whole string, not to index.\n\n**Security**: When exp comes from an untrusted variable, put -- before it so a leading - can't be parsed as a switch. Avoid patterns with nested unbounded quantifiers (e.g. (a+)+) against attacker-controlled input — they can trigger catastrophic backtracking (ReDoS).",
    source: "Tcl regexp(n)",
    examples: "regexp {^[0-9]+$} $input\nregexp -nocase {^error:\\s*(.*)$} $line -> message\nregexp -all -inline {\\S+} $text\nregexp -indices -- {(\\w+)@(\\w+)} $email -> user host\nregexp -about {(a)(b)*c}",
    return_value: "1 if exp matches, 0 otherwise; the total match count instead when -all is given. A list of the match data (instead of writing match variables) when -inline is given. With -about, a two-element {subexpressionCount propertyList} list describing the pattern, without attempting any match.",
};

/// Command spec for `regexp`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        // The match / conversion path is the only one that writes: a failed
        // `regexp`, and a `scan` or `binary scan` whose input runs out, leave
        // each remaining target's previous value in place and never create a
        // target that did not exist. Measured identical on tclsh 8.4.20,
        // 8.5.19, 8.6.18, 9.0.4 and 9.1b0. Without this the store feeding one
        // looked overwritten-before-read and O109 deleted it (#2051).
        traits: Traits::BYTE_COMPILED
            | Traits::FRAME_HASH_BUILTIN
            | Traits::CONDITIONAL_VARIABLE_WRITE,
        // The post-switch positional floor is 2 (`exp`, `string`) in the
        // general case, but `-about` relaxes it to 1 (`exp` alone) —
        // confirmed unchanged from Tcl 8.4 (`core-8-4-20`) through 9.0.4's
        // `Tcl_RegexpObjCmd` (`generic/tclCmdMZ.c`):
        // `if ((objc - i) < (2 - about)) { ...wrong # args... }`, where
        // `about` is 1 when `-about` was given, 0 otherwise. `Arity` has
        // no per-switch axis, so the floor here is the loosest of the two
        // (1, not 2) to avoid a false "too few arguments" on a legitimate
        // `regexp -about exp` call; a plain `regexp somePattern` (missing
        // `string`, no `-about`) is consequently not caught by this
        // coarse check.
        arity: Arity::at_least(1),
        // `Tcl_RegexpObjCmd` uses `TCL_EXACT`: unlike the common
        // Tcl_GetIndexFromObj tables, `-sta` is an error rather than -start.
        prefix_matching: PrefixMatching::Strict,
        return_type: Some(TclType::Int),
        // `-inline` returns the match data instead of writing match
        // variables, and `-about` a two-element {subexpressionCount
        // propertyList}.  `-about` is a guaranteed list in every release 8.4
        // through 9.1; `-inline` is one only once something matches, so the
        // hook types it as unknown.  Either way it is not the int this
        // `return_type` names — without that, iterating a `regexp -all
        // -inline` result drew a shimmer warning claiming the list "has int
        // intrep".
        return_type_hook: Some(ReturnTypeHookId::Regexp),
        // `regexp` writes matched substrings to its capture variables while
        // returning the match *count* (or 0/1).  The captures are strings, not
        // the count, so they must not be typed `Int`.
        var_write_typing: VarWriteTyping::Destructured,
        // Whatever the leading switch layout, a capture target requires the
        // pattern, string, and at least one matchVar.  In particular,
        // `regexp $pattern $string` has no variable write even though the
        // dynamic pattern could begin with `-` and make option selection
        // source-opaque.
        variable_write_min_args: Some(3),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        options: REGEXP_OPTIONS,
        option_effect_families: FAMILIES,
        option_relations: &[INLINE_FORBIDS_MATCH_VARIABLES],
        hover: Some(REGEXP_HOVER),
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        inline_codegen_hook: Some(InlineCodegenHookId::Regexp),
        forms: FORMS,
        arg_role_resolver: Some(regexp_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Pattern, ArgRole::VarWrite],
        analyser_hook: Some(crate::hooks::AnalyserHookId::RegexPatternCapture),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::regex::REGEXP),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the trailing words of a call that *can* carry match variables get
    /// [`ArgRole::VarWrite`].
    ///
    /// `-about` describes the pattern and returns before it looks at a
    /// subject, ignoring every later word; `-inline` returns the match data
    /// as a list and rejects a match variable outright. Measured identical on
    /// tclsh 8.4.20, 8.6.18 and 9.0.4:
    ///
    /// ```text
    /// % puts [regexp -about {(a)} extraarg]
    /// 1 {}
    /// % puts [regexp -inline {a(b)} ab v]
    /// regexp match variables not allowed when using -inline
    /// ```
    ///
    /// Attributing the write anyway deleted a live store — `set v old; regexp
    /// -about {a(b)} somestring v; puts $v` prints `old`, and the optimised
    /// program failed with `can't read "v"` (#2135).
    #[test]
    fn about_and_inline_carry_no_match_variables() {
        for args in [
            ["-about", "a(b)", "somestring", "v"],
            ["-inline", "a(b)", "ab", "v"],
        ] {
            let roles = regexp_arg_roles(&args);
            assert!(
                !roles.iter().any(|(_, role)| *role == ArgRole::VarWrite),
                "{args:?} names no match variable: {roles:?}"
            );
            // The pattern is still placed, and still after the one switch:
            // only the trailing layout changes.
            assert!(
                roles.contains(&(1, ArgRole::Pattern)),
                "{args:?} still has its pattern: {roles:?}"
            );
        }
    }

    /// `-inline` declares `SuppressesRole(VarWrite)`: the generic walk reports
    /// the suppression, and the resolver gives no trailing word a write — the
    /// words are the relation's finding instead (tclsh 8.4–9.1: `regexp
    /// match variables not allowed when using -inline`).
    #[test]
    fn inline_suppresses_the_capture_var_writes() {
        let effects = crate::option_effect::option_effects_with(
            REGEXP_OPTIONS,
            FAMILIES,
            InvocationArguments::literals(&["-all", "-inline", "a(b)", "ab", "v"]),
            0,
            None,
            PrefixMatching::Strict,
        );
        assert!(effects.suppresses(ArgRole::VarWrite), "{effects:?}");
        let roles = regexp_arg_roles(&["-all", "-inline", "a(b)", "ab", "v"]);
        assert_eq!(roles, vec![(2, ArgRole::Pattern)]);
        let relation = spec()
            .option_relations
            .first()
            .copied()
            .expect("the -inline relation");
        assert_eq!(
            relation.message,
            Some("regexp match variables not allowed when using -inline")
        );
    }

    /// `-about` declares `ReservesTrailingWords(1)`: the layout after the
    /// switches is `exp` alone, so the one reserved operand is the pattern
    /// and nothing after it is a match variable (tclsh 8.4–9.1: `regexp
    /// -about {(a)}` is `1 {}`).
    #[test]
    fn about_reserves_one_operand() {
        let effects = crate::option_effect::option_effects_with(
            REGEXP_OPTIONS,
            FAMILIES,
            InvocationArguments::literals(&["-about", "(a)"]),
            0,
            None,
            PrefixMatching::Strict,
        );
        assert_eq!(effects.reserved_trailing_words(), Some(1));
        assert_eq!(effects.option_end, 1);
        assert_eq!(
            regexp_arg_roles(&["-about", "(a)"]),
            vec![(1, ArgRole::Pattern)]
        );
        // An exact-only table: `-abo` is not `-about`, so the layout keeps
        // its two reserved operands and its match variables.
        let abbreviated = regexp_arg_roles(&["-abo", "(a)", "s", "v"]);
        assert!(
            abbreviated.contains(&(3, ArgRole::VarWrite)),
            "{abbreviated:?}"
        );
    }

    /// The switch scan, not a text match, decides — so the `--` terminator and
    /// a value word that looks like a switch both behave as tclsh does.
    #[test]
    fn the_match_variable_layout_follows_the_switch_scan() {
        // `regexp -- -about somestring v`: after `--`, `-about` is the
        // *pattern*, so `v` is a match variable. tclsh 9.0.4 agrees — it
        // prints `old`, because the pattern does not match, not because the
        // word was never a variable.
        let terminated = regexp_arg_roles(&["--", "-about", "somestring", "v"]);
        assert!(
            terminated.contains(&(3, ArgRole::VarWrite)),
            "after `--` the trailing word is a match variable: {terminated:?}"
        );
        // `-start`'s value is not a switch, so a `-inline`-shaped value word
        // must not change the layout either.
        let valued = regexp_arg_roles(&["-start", "2", "(a)", "xxa", "v"]);
        assert!(
            valued.contains(&(4, ArgRole::VarWrite)),
            "`-start`'s value is not a switch: {valued:?}"
        );
        // And an ordinary call is untouched.
        let plain = regexp_arg_roles(&["(x)(y)", "zz", "v", "w"]);
        assert!(
            plain.contains(&(2, ArgRole::VarWrite)) && plain.contains(&(3, ArgRole::VarWrite)),
            "every trailing word of a plain call is a match variable: {plain:?}"
        );
    }

    /// Membership pin against Tcl 9.0.4 `Tcl_RegexpObjCmd`
    /// (`generic/tclCmdMZ.c` options table): the exact switch set, with
    /// `-start` the only value-taking switch.
    #[test]
    fn options_match_tcl9_regexp_switch_table() {
        let s = spec();
        let mut names: Vec<&str> = s.options.iter().map(|o| o.name).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "--",
                "-about",
                "-all",
                "-expanded",
                "-indices",
                "-inline",
                "-line",
                "-lineanchor",
                "-linestop",
                "-nocase",
                "-start",
            ]
        );
        for option in s.options {
            assert_eq!(
                option.takes_value(),
                option.name == "-start",
                "{}",
                option.name
            );
        }
    }
}
