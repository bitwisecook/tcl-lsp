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
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
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
        forms: FORMS,
        arg_role_resolver: Some(regexp_arg_roles),
        ..CommandSpec::DEFAULT
    }
}
