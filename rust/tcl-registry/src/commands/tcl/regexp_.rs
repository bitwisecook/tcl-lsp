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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regexp ?switches? exp string ?matchVar? ?subMatchVar ...?",
}];

/// `regexp ?switches? exp string ?matchVar ...?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), arg 0 is the
/// pattern, arg 1 the string, and args 2+ are capture variables.  Resolve
/// `VarWrite` for every trailing capture var dynamically (the leading-option
/// shift means a static slot list cannot place them).
fn regexp_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    // Leading switches (`-start` consumes a value; `--` terminates) are skipped
    // via the shared, arity-aware scanner driven by `REGEXP_OPTIONS`, so the
    // value-taking set lives in exactly one place.
    let i = first_positional_index(REGEXP_OPTIONS, args, 0);
    let capture_start = i + 2; // skip pattern + string
    (capture_start..args.len())
        .filter_map(|j| u8::try_from(j).ok().map(|j| (j, ArgRole::VarWrite)))
        .collect()
}

/// A boolean switch (`-flag`) — takes no value, available in all dialects.
const fn flag(name: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    }
}

/// The 11 `regexp` switches. `-start` takes an `index` value; the rest are
/// boolean flags (`--` terminates option parsing).
const REGEXP_OPTIONS: &[OptionSpec] = &[
    flag("-nocase"),
    flag("-expanded"),
    flag("-line"),
    flag("-linestop"),
    flag("-lineanchor"),
    flag("-all"),
    flag("-inline"),
    flag("-indices"),
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    flag("-about"),
    flag("--"),
];

/// Hover documentation for `regexp`.
const REGEXP_HOVER: HoverSnippet = HoverSnippet {
    summary: "Match a regular expression against a string.",
    synopsis: &["regexp ?switches? exp string ?matchVar? ?subMatchVar ...?"],
    snippet: "Returns 1 if *exp* matches part of *string*, 0 otherwise. Matching substrings are stored in *matchVar* and *subMatchVar*.\n\n**Security**: Use `--` before the pattern when it comes from a variable to prevent option injection. Avoid nested quantifiers like `(a+)+` which can cause catastrophic backtracking (ReDoS) on crafted input.",
    source: "Tcl regexp(1)",
    examples: "",
    return_value: "1 if the pattern matches, 0 otherwise.",
};

/// Command spec for `regexp`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp",
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::at_least(1),
        return_type: Some(TclType::Int),
        // `regexp` writes matched substrings to its capture variables while
        // returning the match *count* (or 0/1).  The captures are strings, not
        // the count, so they must not be typed `Int` (issue #867).
        var_write_typing: VarWriteTyping::Destructured,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
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
