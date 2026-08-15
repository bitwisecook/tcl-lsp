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
fn regexp_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let i = first_positional_index(REGEXP_OPTIONS, args, 0);
    let capture_start = i + 2; // skip pattern + string
    std::iter::once((i, ArgRole::Pattern))
        .chain((capture_start..args.len()).map(|index| (index, ArgRole::VarWrite)))
        .filter_map(|(index, role)| u8::try_from(index).ok().map(|index| (index, role)))
        .collect()
}

/// A boolean switch (`-flag`) — takes no value, available in all dialects.
const fn flag(name: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::flag(),
        detail,
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    }
}

/// The 11 `regexp` switches — confirmed byte-for-byte stable (same 11
/// names, same value-taking shape) across the fetched Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1 manpages, so none carries a `dialects:` restriction.
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
    flag(
        "-inline",
        "Return the match data as a list instead of writing match variables (illegal to combine with a matchVar/subMatchVar argument). With -all, every match's data is concatenated into one flat list.",
    ),
    flag(
        "-indices",
        "Store each matchVar/subMatchVar as a {first last} character-index pair into string instead of the matched text.",
    ),
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "Character index into string to start matching at. Tcl 8.4 accepts only a plain non-negative integer; Tcl 8.5 and later accept the fuller index syntax used by string index (e.g. end, end-N). ^ no longer anchors to the string's real start there, though \\A still does; -indices results stay relative to the whole string, and index is clamped to the string's bounds.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    flag(
        "-about",
        "Skip matching and instead return {subexpressionCount propertyList} describing the compiled pattern, for debugging; needs only exp — string may be omitted.",
    ),
    flag(
        "--",
        "Ends switch parsing; the next word is treated as exp even if it begins with -.",
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
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
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
        // `regexp` writes matched substrings to its capture variables while
        // returning the match *count* (or 0/1).  The captures are strings, not
        // the count, so they must not be typed `Int` (issue #867).
        var_write_typing: VarWriteTyping::Destructured,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        options: REGEXP_OPTIONS,
        hover: Some(REGEXP_HOVER),
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        inline_codegen_hook: Some(InlineCodegenHookId::Regexp),
        forms: FORMS,
        arg_role_resolver: Some(regexp_arg_roles),
        analyser_hook: Some(crate::hooks::AnalyserHookId::RegexPatternCapture),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
