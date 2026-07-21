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

//! `switch` — pattern-based branching on a subject string.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "switch ?options? string pattern body ?pattern body ...?",
    dialects: None,
}];

/// Options that consume a following value argument.
const SWITCH_VALUE_OPTIONS: &[&str] = &["-matchvar", "-indexvar"];

/// Dynamic arg role resolver for `switch`.
///
/// Skips option flags (including value-consuming options like
/// `-matchvar`/`-indexvar`), then identifies pattern/body pairs
/// or a single braced-list body.
fn switch_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut i: usize = 0;
    // Skip option flags.
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if !a.starts_with('-') {
            break;
        }
        if SWITCH_VALUE_OPTIONS.contains(&a) {
            i += 2;
        } else {
            i += 1;
        }
    }
    // Skip switch value.
    if i < args.len() {
        i += 1;
    }
    if i >= args.len() {
        return Vec::new();
    }
    let mut roles = Vec::new();
    // Braced list form: single trailing argument.
    if i == args.len() - 1 {
        if let Ok(idx) = u8::try_from(i) {
            roles.push((idx, ArgRole::Body));
        }
        return roles;
    }
    // List form: pattern body pairs.
    while i + 1 < args.len() {
        if args[i + 1] != "-"
            && let Ok(idx) = u8::try_from(i + 1)
        {
            roles.push((idx, ArgRole::Body));
        }
        i += 2;
    }
    roles
}

/// Command spec for `switch`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "switch",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::HAS_SWITCH_BODY,
        // `string pattern body ?pattern body ...?` (an odd count from 3)
        // — OR the single-braced-body shorthand, `string {pattern body
        // ...}` (exactly 2) — confirmed against tclsh 8.6.14: `switch $s
        // a b c` (3 args, an unpaired trailing pattern) fails "wrong #
        // args", but `switch $s {a b}` (2 args, the braced form) and
        // `switch $s a b c d` (4, two full pairs) both succeed.
        arity: Arity::stepped(3, Arity::UNLIMITED, 2).with_also_exact(2),
        arg_role_resolver: Some(switch_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Switch),
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-exact",
                    value: OptionValue::flag(),
                    detail: "Exact string compare mode.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-glob",
                    value: OptionValue::flag(),
                    detail: "Glob pattern mode.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-regexp",
                    value: OptionValue::flag(),
                    detail: "Regular expression mode.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-nocase",
                    value: OptionValue::flag(),
                    detail: "Case-insensitive matching.",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-matchvar",
                    value: OptionValue::value("varName"),
                    detail: "Store match in variable (regexp mode).",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-indexvar",
                    value: OptionValue::value("varName"),
                    detail: "Store match indices in variable (regexp mode).",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "End of options.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Pattern-based branching on a subject string.",
            synopsis: &[
                "switch ?options? string pattern body ?pattern body ...?",
                "switch ?options? string {pattern body ?pattern body ...?}",
            ],
            snippet: "Use `-exact`, `-glob`, or `-regexp` to select matching mode.",
            source: "Tcl switch(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        // `TclNRSwitchObjCmd` (generic/tclCmdMZ.c) only scans for `-flag`
        // words up to `objc - 2`: the trailing `string` and
        // pattern-list-or-first-pattern words are never mistaken for
        // options, even when dynamic/tainted and starting with `-`.
        reserved_trailing_words: 2,
        case_list: Some(&CaseListSpec::SWITCH),
        analyser_hook: Some(crate::hooks::AnalyserHookId::Switch),
        ..CommandSpec::DEFAULT
    }
}
