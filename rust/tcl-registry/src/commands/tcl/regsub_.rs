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

//! `regsub` — perform substitutions based on regular expression matching.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regsub ?switches? exp string subSpec ?varName?",
}];

/// `regsub ?switches? exp string subSpec ?varName?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), the positional args
/// are `exp` (0), `string` (1), `subSpec` (2), and the optional `varName` (3).
/// When `varName` is present it names the variable the result is written to;
/// resolve it as `VarWrite` dynamically (the leading-option shift means a
/// static slot cannot place it).  Omitting `varName` (Tcl 8.7+/9 returns the
/// substituted string instead) simply yields no `VarWrite` index.
fn regsub_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
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
    // exp (i), string (i+1), subSpec (i+2), varName (i+3).
    let var_idx = i + 3;
    (var_idx < args.len())
        .then(|| u8::try_from(var_idx).ok().map(|v| (v, ArgRole::VarWrite)))
        .flatten()
        .into_iter()
        .collect()
}

/// `regsub -command ?switches? exp string cmdPrefix ?varName?` (Tcl 9.0+, TIP
/// 463): with `-command` present, the `subSpec` positional (index `i+2` after
/// the leading switches) is not a replacement template but a command prefix
/// called once per match with the whole match + capture-group substrings
/// appended — a variadic count (`AtLeast(1)`: the whole match is always
/// passed). Without `-command`, `subSpec` is an ordinary string (no prefix).
fn regsub_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    let mut i = 0;
    let mut has_command = false;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            // `-command` and its unambiguous abbreviations (`-c`..`-comman`);
            // no other regsub switch begins with `-c`.
            if a.len() >= 2 && "-command".starts_with(a) {
                has_command = true;
            }
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    let sub_idx = i + 2;
    if has_command && sub_idx < args.len() {
        u8::try_from(sub_idx)
            .map(|s| vec![(s, AppendedArity::AtLeast(1))])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Command spec for `regsub`.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-expanded",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-line",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-linestop",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-lineanchor",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `regsub -command` is Tcl 9.0+ (TIP 463).
    OptionSpec {
        name: "-command",
        value: OptionValue::flag(),
        detail: "Treat subSpec as a command prefix to call per match.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regsub",
        byte_array_effect: ByteArrayEffect::Coerces,
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::new(3, 4),
        return_type: Some(TclType::Int),
        // The `varName` form writes the substituted *string* to its target
        // while returning the replacement *count*.  The result is always a
        // string (not a format-/element-dependent piece like `scan`/`lassign`),
        // so type it `String` — that keeps real string-in-arithmetic shimmer
        // diagnostics while avoiding the old bogus `Int` (issue #867).
        var_write_typing: VarWriteTyping::Fixed(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Perform substitutions based on regular expression matching.",
            synopsis: &["regsub ?switches? exp string subSpec ?varName?"],
            snippet: "Matches *exp* against *string* and replaces the matched portion with *subSpec*. With `-all`, replaces all occurrences.\n\n**Security**: Use `--` before the pattern when it comes from a variable to prevent option injection. The *subSpec* supports `\\0`..`\\9` backreferences and `&` for the full match.",
            source: "Tcl regsub(1)",
            examples: "",
            return_value: "The substituted string (Tcl 8.5+), or the count of replacements when *varName* is given.",
        }),
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        arg_role_resolver: Some(regsub_arg_roles),
        command_prefix_resolver: Some(regsub_command_prefixes),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::RegexPatternCapture),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Membership pin against Tcl 9.0.4 `Tcl_RegsubObjCmd`
    /// (`generic/tclCmdMZ.c` options table): the exact switch set, with
    /// `-start` the only value-taking switch (`-command` is a flag that
    /// changes how `subSpec` is interpreted).
    #[test]
    fn options_match_tcl9_regsub_switch_table() {
        let s = spec();
        let mut names: Vec<&str> = s.options.iter().map(|o| o.name).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "--",
                "-all",
                "-command",
                "-expanded",
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
