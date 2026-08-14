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
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

// Both forms are documented, word-for-word identically (modulo the
// `string`/`value` placeholder rename — see the `spec()` doc comment
// below), on every one of the five fetched manpages (8.4, 8.5, 8.6, 9.0,
// 9.1): the flat pattern/body-pair form and the single-braced-list
// shorthand. Neither form is dialect- or version-restricted.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "switch ?options? string pattern body ?pattern body ...?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "switch ?options? string {pattern body ?pattern body ...?}",
        ..FormSpec::DEFAULT
    },
];

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
///
/// -nocase, -matchvar, and -indexvar were added in Tcl 8.5 — Tcl 8.4's
/// switch(n) SYNOPSIS/OPTIONS document only -exact, -glob, -regexp, and
/// --. Tcl 8.5 also newly documents that a call with exactly two
/// arguments in total is never scanned for options at all (so `switch
/// -weird {…}`, a subject that happens to look like a flag, needs no --
/// there); Tcl 8.4's page has no such exception and scans every leading
/// `-`-prefixed word as an option regardless of the total count. Tcl 8.5,
/// 8.6, and 9.0's SYNOPSIS/DESCRIPTION/OPTIONS wording is identical
/// (fetched and diffed directly, not paraphrased) — 8.6's and 9.0's
/// rendered bodies are byte-for-byte identical to each other, and 8.5
/// differs from both only in typesetting (an ASCII hyphen vs Unicode em
/// dash in the NAME line, blank-line spacing, and copyright-block
/// ordering), never in wording — so no 8.6 or 9.0 delta exists for this
/// command beyond 8.5's. Tcl 9.1 (whose own manpage self-identifies as
/// version "9.1b0", a live beta) adds a fourth match mode, -integer,
/// absent from every earlier version's SYNOPSIS/OPTIONS list, and amends
/// -nocase's own wording to note it cannot be combined with -integer;
/// 9.1's page also renames the placeholder argument from "string" to
/// "value" throughout (SYNOPSIS and body text alike) — a wording-only
/// change with no syntactic or behavioural effect, which this spec keeps
/// spelled "string" for continuity with every other version's
/// forms/hover text.
/// `switch`'s option table, hoisted out of the spec literal so the
/// builder stays inside the line budget.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-exact",
        value: OptionValue::flag(),
        detail: "Exact string compare mode. This is the default.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Glob-style pattern mode, as implemented by `string match`.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Integer comparison mode: string and every pattern (other than a trailing default) must be a valid integer, or switch raises an error. Cannot be combined with -nocase.",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Regular expression pattern mode.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive matching. Not supported together with -integer (Tcl 9.1+).",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-matchvar",
        value: OptionValue::value("varName"),
        detail: "Store the list of matched substrings here — element 0 is the overall match, each later element a capturing group (only legal with -regexp); an empty list when a default branch runs.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-indexvar",
        value: OptionValue::value("varName"),
        detail: "Store the list of matched substring start/end index pairs here, parallel to -matchvar (only legal with -regexp); an empty list when a default branch runs.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "End of options: the next word is always the subject string, even if it starts with -.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "switch",
        // Present, unrestricted, and available in every dialect including
        // iRules: its own `dialects` group explicitly carries the `IRULES`
        // bit (the `ALL_TCL | IRULES` value below), so it intersects the
        // bare `IRULES` mask directly rather than being kept out — there is
        // no disable list. A pure control-flow primitive with no
        // filesystem/process/network access, so every dialect that hosts a
        // real Tcl core (irules, iapps, tmsh, the EDA shells, expect, tk,
        // itcl) carries it unmodified. Its *legal
        // option set* narrows per Tcl version through the individual
        // OptionSpecs' own `dialects` gates below (-nocase/-matchvar/
        // -indexvar from 8.5, -integer from 9.1) — not a whole-command
        // dialect gate.
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
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
        // `switch $s a b c d` (4, two full pairs) both succeed. This
        // two-shape grammar itself is unchanged across all five fetched
        // manpages (8.4-9.1) — only the recognised *option* vocabulary
        // that precedes it is version-gated (see the per-option
        // `dialects` below).
        arity: Arity::stepped(3, Arity::UNLIMITED, 2).with_also_exact(2),
        arg_role_resolver: Some(switch_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Switch),
        return_type: Some(TclType::String),
        options: OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Pattern-based branching on a subject string.",
            synopsis: &[
                "switch ?options? string pattern body ?pattern body ...?",
                "switch ?options? string {pattern body ?pattern body ...?}",
            ],
            snippet: "Compares string against each pattern in turn and, on the first match, evaluates the corresponding body and returns its result. -exact (the default), -glob (as `string match`), -regexp, and, from Tcl 9.1, -integer select the comparison mode; -nocase (Tcl 8.5+) makes any of them case-insensitive except -integer, which it cannot combine with — -integer also turns a non-integer string or pattern, other than a trailing default, into an error. A pattern of default, which must come last, matches unconditionally; with no match and no default, switch returns an empty string. A body of exactly \"-\" shares the next pattern's body, so several patterns can fall through to one script. Patterns and bodies may be given as separate words, with substitutions applying normally, or grouped into one braced {pattern body ...} argument, handy for multi-line switches — no command or variable substitution happens on patterns in the braced form, so it can behave differently from the separate-words form. -matchvar and -indexvar (Tcl 8.5+, only with -regexp) capture the overall match plus each capturing group's substrings, or their start/end indices, into the named variables, writing an empty list when a default branch runs. When switch is called with exactly two arguments in total, neither is scanned as an option (documented from Tcl 8.5 on) — the first is always the subject and the second the pattern list, even if the subject begins with -; with more arguments, a subject beginning with - still needs --. Tcl 8.4 recognises only -exact/-glob/-regexp/--.",
            source: "Tcl switch(n)",
            examples: "switch -glob -- $ext {\n    .tcl    -\n    .tm     { puts \"Tcl source\" }\n    .h      { puts \"C header\" }\n    default { puts \"unknown extension\" }\n}\n\n# Capture what matched (Tcl 8.5+)\nswitch -regexp -matchvar parts -- $line {\n    {^(\\w+):\\s*(.*)$} {\n        puts \"key=[lindex $parts 1] value=[lindex $parts 2]\"\n    }\n}\n\n# Dispatch on argument count (Tcl 9.1+)\nswitch -integer -- [llength $args] {\n    0       { puts \"no arguments\" }\n    1       { puts \"one argument: [lindex $args 0]\" }\n    default { puts \"many arguments: $args\" }\n}",
            return_value: "The result of evaluating the matched body, or an empty string when no pattern matches and there is no default clause.",
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
