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
    dialects: None,
}];

/// `regsub ?switches? exp string subSpec ?varName?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), the positional args
/// are `exp` (0), `string` (1), `subSpec` (2), and the optional `varName` (3).
/// When `varName` is present it names the variable the result is written to;
/// resolve it as `VarWrite` dynamically (the leading-option shift means a
/// static slot cannot place it).  `varName` has always been optional — the
/// fetched 8.4 through 9.1 manpages agree verbatim ("... or returns *string*
/// if *varName* is not present") that omitting it returns the substituted
/// string directly instead of writing a variable and returning the
/// replacement count — so omitting it simply yields no `VarWrite` index,
/// with no dialect/version gating of its own.
fn regsub_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut i = 0;
    let mut has_command = false;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            has_command |= a == "-command";
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    // exp (i), string (i+1), subSpec (i+2), varName (i+3).
    let mut roles: Vec<(u8, ArgRole)> = Vec::new();
    let push = |roles: &mut Vec<(u8, ArgRole)>, idx: usize, role: ArgRole| {
        if idx < args.len()
            && let Ok(idx) = u8::try_from(idx)
        {
            roles.push((idx, role));
        }
    };
    // The regular expression itself — the leading-option shift means a static
    // slot cannot place it, which is why the LSP used to re-scan the options
    // itself (issue #1185).
    push(&mut roles, i, ArgRole::Pattern);
    // The replacement template, whose `\&` / `\N` backreferences are the
    // `FormatType::Regsub` mini-language.  With `-command` the same position
    // is a callback prefix instead (handled by `regsub_command_prefixes`), so
    // it carries no template role then.
    if !has_command {
        push(&mut roles, i + 2, ArgRole::FormatString);
    }
    push(&mut roles, i + 3, ArgRole::VarWrite);
    roles
}

/// `regsub -command ?switches? exp string cmdPrefix ?varName?` (Tcl 9.0+, TIP
/// 463 "Command-Driven Substitutions for regsub"): with `-command` present,
/// the `subSpec` positional (index `i+2` after the leading switches) is not
/// a replacement template but a command prefix called once per match with
/// the whole match + capture-group substrings appended — a variadic count
/// (`AtLeast(1)`: the whole match is always passed; the number of capture
/// groups, and hence the exact total, depends on `exp`). Without
/// `-command`, `subSpec` is an ordinary string (no prefix). `-command` is
/// confirmed absent from the fetched 8.4, 8.5, and 8.6 switch lists and
/// present, identically, in the fetched 9.0 and 9.1 manpages.
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
            // No abbreviation: Tcl's option parser resolves regsub's switch
            // table with `Tcl_GetIndexFromObj(..., TCL_EXACT, ...)`
            // (confirmed in the fetched Tcl 9.0.4 `generic/tclCmdMZ.c`
            // `Tcl_RegsubObjCmd`), and empirically in live tclsh 8.6.14:
            // `regsub -noc {a} aaa X y` errors with `bad option "-noc"`
            // rather than matching `-nocase`. Only the exact spelling
            // `-command` enables command-prefix mode.
            if a == "-command" {
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

/// A boolean switch (`-flag`) — takes no value, available in all dialects
/// (the common case for this file's switches; `-command` below overrides).
const fn flag(name: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::flag(),
        detail,
        dialects: None,
        aliases: &[],
        min_version: None,
    }
}

/// The `regsub` switches — confirmed byte-for-byte stable (same names, same
/// value-taking shape, same documented behaviour) across the fetched Tcl
/// 8.4, 8.5, 8.6, 9.0, and 9.1 manpages, with exactly one exception:
/// `-command`, added in 9.0 (TIP 463) and absent from 8.4/8.5/8.6. `-start`
/// is the only *universal* switch that takes a value (an `index`), but its
/// value *grammar* is a genuine behavioural change, not merely added prose
/// (the 8.4 manpage lacks the "same manner as ... string index" sentence
/// 8.5 onward adds): `generic/tclCmdMZ.c`'s `Tcl_RegsubObjCmd` parses the
/// value with plain `Tcl_GetIntFromObj` (a literal non-negative integer
/// only; `end`/`end-N` fails) in Tcl 8.4 (`core-8-4-20`), and with
/// `TclGetIntForIndexM` (the same `end`/`end-N` grammar `string index`
/// uses) from 8.5 (`core-8-5-19`) through 9.1 (`core-9-1-b0`) — confirmed
/// directly against the fetched C source at all five tags, mirroring
/// `regexp`'s identical `-start` split (see `regexp_.rs`). See the
/// `-start` `OptionSpec`'s `detail` text below. Every other switch besides
/// `-command` is a boolean flag. `--` terminates option parsing.
const OPTIONS: &[OptionSpec] = &[
    flag(
        "-nocase",
        "Match case-insensitively: upper-case characters in string are treated as lower case when matching against exp; substitutions written into subSpec still use the original, unconverted text of string.",
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
        "Replace every non-overlapping match of exp in string instead of only the first; & and \\n backreferences in subSpec are recomputed independently for each match.",
    ),
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "Character index into string to start matching at. Tcl 8.4 accepts only a plain non-negative integer; Tcl 8.5 and later accept the fuller index syntax used by string index (e.g. end, end-N). ^ no longer anchors to the string's real start there, though \\A still does; index is clamped to the string's bounds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `regsub -command` is Tcl 9.0+ (TIP 463): absent from the fetched
    // 8.4/8.5/8.6 manpages' switch lists, present — identically worded —
    // in the fetched 9.0 and 9.1 manpages.
    OptionSpec {
        name: "-command",
        value: OptionValue::flag(),
        detail: "Treat subSpec as a command prefix (a non-empty list) instead of a substitution template: & and \\n lose their special meaning. The whole match, then each capturing subexpression's match (like regexp -inline), are appended to the prefix, and the completed list is evaluated as a Tcl command whose result becomes the replacement text. Invoked once per match with -all, otherwise at most once; any error or exception from the callback becomes an error from regsub itself.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    flag(
        "--",
        "Ends switch parsing; the next word is treated as exp even if it begins with -.",
    ),
];

/// Command spec for `regsub`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regsub",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        byte_array_effect: ByteArrayEffect::Coerces,
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
        // Positional floor/ceiling — `exp`, `string`, `subSpec` required
        // (3), optional `varName` (4). Confirmed identical in the fetched
        // 8.4, 8.5, 8.6, 9.0, and 9.1 manpages' `regsub ?switches? exp
        // string subSpec ?varName?` synopsis; `-command` (9.0+) changes how
        // `subSpec` is interpreted, not the word count, so the range is
        // unaffected by it.
        arity: Arity::new(3, 4),
        // Documented return is the int replacement count (the `varName`
        // form). The `varName`-omitted form — equally idiomatic, e.g.
        // `set new [regsub $pat $orig $repl]` — actually returns the
        // substituted *string* instead: a per-form refinement deferred the
        // same way `scan`'s inline (no-`varName`) form is (see
        // `commands/tcl/scan_.rs`). True in every fetched release from 8.4
        // through 9.1 alike — the manpage's "returns *string* if *varName*
        // is not present" sentence is unchanged across all five.
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
            dialects: None,
        }],
        options: OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Perform substitutions based on regular expression pattern matching.",
            synopsis: &["regsub ?switches? exp string subSpec ?varName?"],
            snippet: "Matches the regular expression exp (an ARE — see re_syntax) against string and replaces the matched text with subSpec. Only the first match is replaced unless -all is given, which replaces every non-overlapping match instead. Within subSpec, & or \\0 stands for the whole match and \\1 through \\9 for the text of the corresponding parenthesized subexpression; extra backslashes escape these forms, and enclosing subSpec in braces is usually safest when it needs to contain backslashes of its own.\n\nWhen varName is supplied, the substituted text is stored there and the command returns the number of replacements made; when it is omitted, the substituted string is returned directly instead. Since Tcl 9.0 (TIP 463), -command changes subSpec from a template into a command prefix: the whole match plus each capturing group's text are appended to it (as regexp -inline would return them), and the prefix is evaluated as a Tcl command whose result becomes the replacement text — useful when the substitution needs computation a plain template can't express.\n\n**Security**: put -- before exp when it comes from an untrusted variable so a leading - can't be parsed as a switch. Avoid patterns with nested unbounded quantifiers (e.g. (a+)+) against attacker-controlled input — they can trigger catastrophic backtracking (ReDoS).",
            source: "Tcl regsub(n)",
            examples: "regsub -all {\\mfoo\\M} $text bar text\nregsub -nocase {\\yinteresting\\y} $text {\"&\"} text\nset trimmed [regsub -all {^\\s+|\\s+$} $line {}]\nregsub -all -command {\\w+} $message {string totitle}",
            return_value: "The substituted string, or the count of replacements made when varName is given.",
        }),
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        arg_role_resolver: Some(regsub_arg_roles),
        // The `subSpec` replacement template's own mini-language; the
        // `ArgRole::FormatString` the resolver puts on that word locates it.
        format_string_type: Some(FormatType::Regsub),
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

    /// `regsub`'s option table is resolved with `Tcl_GetIndexFromObj(...,
    /// TCL_EXACT, ...)` (Tcl 9.0.4 `generic/tclCmdMZ.c`
    /// `Tcl_RegsubObjCmd`) — confirmed live against tclsh 8.6.14, where
    /// `regsub -noc {a} aaa X y` errors with `bad option "-noc"` rather
    /// than matching `-nocase`. `-command` gets no special treatment: an
    /// abbreviation must NOT flip `regsub_command_prefixes` into
    /// command-prefix mode, only the exact spelling may.
    #[test]
    fn command_prefix_resolver_requires_exact_command_spelling() {
        // Exact `-command` at the front: `subSpec` (index 3: -command, exp,
        // string, subSpec) is a command prefix appending >= 1 arg.
        assert_eq!(
            regsub_command_prefixes(&["-command", "exp", "string", "prefix"]),
            vec![(3, AppendedArity::AtLeast(1))]
        );
        // Any abbreviation of `-command` is a distinct (invalid-in-real-Tcl)
        // switch spelling, not `-command` itself, so it must resolve no
        // command prefix at all -- an over-eager prefix match here would
        // mismodel a script real Tcl rejects with "bad option".
        for abbrev in ["-c", "-co", "-com", "-comm", "-comma", "-comman"] {
            assert_eq!(
                regsub_command_prefixes(&[abbrev, "exp", "string", "prefix"]),
                Vec::new(),
                "{abbrev} must not enable command-prefix mode"
            );
        }
    }
}
