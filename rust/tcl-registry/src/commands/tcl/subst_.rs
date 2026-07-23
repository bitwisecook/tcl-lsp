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

//! `subst` — perform Tcl substitutions on a string.
use crate::prelude::*;

/// `subst`'s embedded `[command]` substitutions can do literally anything
/// once evaluated, the same "unknowable statically" reasoning `eval` /
/// `apply` document on their own `Unknown` side effects — except `subst`
/// only evaluates a command when one is actually embedded in `string`
/// *and* command substitution is enabled for this call (see
/// `subst_evaluates_commands`/`taint_sink_gate` below), whereas `eval`
/// unconditionally does. `reads`/`writes` stay `true` rather than `eval`'s
/// `false`/`false` to match the sibling body-evaluating constructs that
/// keep an unknown-but-present effect (`if`, `for`, `foreach`, `catch`)
/// rather than a no-effect placeholder.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

// `-nobackslashes`/`-nocommands`/`-novariables` are the only recognised
// switches in Tcl 8.4, 8.5, 8.6, and 9.0 — byte-for-byte identical
// SYNOPSIS/DESCRIPTION wording in all four fetched manpages, each simply
// disabling its named substitution (all three run by default). Tcl 9.1
// keeps these three unchanged and adds three positive counterparts
// (`-backslashes`/`-commands`/`-variables`) as a second SYNOPSIS line: with
// any of these present, every substitution defaults *off* and only the
// named one(s) turn on. The 9.1 manpage states the two families are
// interchangeable in matched pairs (`subst -nobackslashes -nocommands
// $string` and `subst -variables $string` are called equivalent) and that
// "it is not allowed to combine positive and negated options" in one call.
// For the negative family, confirmed live on `tclsh` 8.6.14 that a word
// which is neither a recognised switch nor the final `string` operand is a
// hard `bad option "…": must be -nobackslashes, -nocommands, or
// -novariables` error (no `--` terminator recognised) — no 9.1 interpreter
// was available locally to empirically re-confirm the positive family
// parses the same way, but the manpage describes both families through the
// same switch-table convention.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nobackslashes",
        value: OptionValue::flag(),
        detail: "Disable backslash substitution; backslash sequences such as \\n are left as literal text. Command and variable substitution are unaffected.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocommands",
        value: OptionValue::flag(),
        detail: "Disable top-level command substitution; [ and ] are treated as ordinary characters. A [command] nested inside a variable reference (e.g. an array index) still runs, since that is part of resolving the variable, not a top-level substitution.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-novariables",
        value: OptionValue::flag(),
        detail: "Disable top-level variable substitution; $ is treated as an ordinary character. A $variable reference nested inside a command still runs, since that is part of evaluating the command, not a top-level substitution.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // Tcl 9.1 adds positive forms that enable *only* the named
    // substitution, defaulting every other kind off. Positive and negated
    // options may not be combined in one call.
    OptionSpec {
        name: "-backslashes",
        value: OptionValue::flag(),
        detail: "Enable only backslash substitution; commands and variables are left as literal text (Tcl 9.1). Cannot combine with -nobackslashes/-nocommands/-novariables.",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-commands",
        value: OptionValue::flag(),
        detail: "Enable only command substitution; backslashes and variables are left as literal text (Tcl 9.1). Cannot combine with -nobackslashes/-nocommands/-novariables.",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-variables",
        value: OptionValue::flag(),
        detail: "Enable only variable substitution; backslashes and commands are left as literal text (Tcl 9.1). Cannot combine with -nobackslashes/-nocommands/-novariables.",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
];

// Tcl 8.4, 8.5, 8.6, and 9.0 document exactly one SYNOPSIS line (the
// negative-switch form). Tcl 9.1 adds a second SYNOPSIS line for the new
// positive-switch form (see the OPTIONS comment above) — narrowed to
// `TCL91` here rather than inheriting the command's own unrestricted
// `dialects: None`, mirroring how `return_.rs` narrows its `-level` form to
// `TCL85_PLUS`.
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "subst ?-backslashes? ?-commands? ?-variables? string",
        dialects: Some(DialectSet::TCL91),
    },
];

/// Fold a literal `subst string`.
///
/// `subst` performs variable, command, and backslash substitution on
/// its string argument — even inside braces (`subst {$x}` substitutes
/// `$x`).  The Rust O129 path hands this the *raw* literal argument (it
/// has no upstream `$var` resolution — that is the deferred B2 work), so
/// to stay sound we fold only the bare `subst string` form whose string
/// carries **no** substitution: no `$`, `[`, or `\`.  Such a string is
/// its own `subst` result.  Anything with a substitution bails
/// (a stricter subset of what a fold with upstream resolution +
/// the `[command]` caller-bail could decide).  The option forms
/// (`-nobackslashes` / …) change which substitutions apply, so the
/// multi-arg form bails too.
fn fold_subst(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    if s.contains(['$', '[', '\\']) {
        return None;
    }
    Some((*s).to_owned())
}

/// Whether a `subst` call — given its argument words, excluding the
/// command name — performs *command* substitution, the only hazard T100
/// warns about for this command (variable and backslash substitution alone
/// cannot execute anything). Drives `CommandSpec::taint_sink_gate`, so
/// `subst -nocommands $tainted` — the exact mitigation this command's own
/// hover snippet recommends — no longer trips the code-injection sink it
/// was written to avoid.
///
/// The legacy negative options (`-nobackslashes`/`-nocommands`/
/// `-novariables`) default every substitution *on*, each disabling one;
/// `-nocommands` anywhere disables command substitution outright. Tcl
/// 9.1's positive options (`-backslashes`/`-commands`/`-variables`) invert
/// that: default every substitution *off*, each enabling one, so command
/// substitution then runs only when `-commands` is itself present. Tcl
/// rejects mixing the two families in one call, so seeing any positive
/// flag switches this scan to positive mode; option scanning stops at the
/// first non-flag word (the `string` operand).
fn subst_evaluates_commands(args: &[&str]) -> bool {
    let mut positive_mode = false;
    let mut nocommands = false;
    let mut commands = false;
    for &a in args {
        match a {
            "-commands" => {
                commands = true;
                positive_mode = true;
            }
            "-backslashes" | "-variables" => positive_mode = true,
            "-nocommands" => nocommands = true,
            "--" => break,
            _ if a.starts_with('-') => {}
            _ => break,
        }
    }
    if positive_mode { commands } else { !nocommands }
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "subst",
        // Present, unrestricted, and on no dialect's `disabled_commands`
        // list (only `f5-irules` has a non-empty one, and `subst` isn't on
        // it — checked against `tcl-dialect/src/profile.rs`'s
        // `IRULES_DISABLED_COMMANDS`): every dialect hosting a real
        // embedded Tcl core (irules, iapps, tmsh, the EDA shells, expect,
        // tk, itcl) carries this command unmodified. Its legal *option*
        // set still narrows per Tcl version through each OptionSpec's own
        // `dialects` gate above (enforced generically by
        // `ProfileQueries::is_option_available` together with each
        // profile's `version_ceiling`), which is why the 9.1-only positive
        // switches can never resolve under any of those 8.x-pinned
        // embedded cores even though the command itself is universal.
        dialects: None,
        byte_array_effect: ByteArrayEffect::Coerces,
        traits: Traits::TAINT_SINK | Traits::IS_UNESCAPE | Traits::PERFORMS_SUBSTITUTION,
        // Exactly one trailing `string` is mandatory; 0 or more recognised
        // switch words may precede it with no fixed ceiling (a switch may
        // legally repeat — `subst -nocommands -nocommands $s` is valid,
        // confirmed live on `tclsh` 8.6.14). Identical arity shape in all
        // five fetched manpages, so `at_least(1)` with no upper bound is
        // correct for every version.
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold: Some(fold_subst),
        hover: Some(HoverSnippet {
            summary: "Perform backslash, command, and variable substitutions on a string.",
            synopsis: &[
                "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
                "subst ?-backslashes? ?-commands? ?-variables? string",
            ],
            snippet: "Applies the same variable, command, and backslash substitution rules Tcl itself uses when parsing a command word, but a second time: string is substituted once by the Tcl parser when the call to subst is itself parsed, then again by subst at runtime. This is what lets `subst {$x}` substitute `$x` even though the enclosing braces would normally suppress substitution.\n\nAll three substitution kinds run by default. -nobackslashes, -nocommands, and -novariables (every Tcl version) each turn off exactly the substitution they name; -nocommands is the one that matters for safety, since it is the only kind that can execute arbitrary code. One kind can still trigger another even when disabled: with -novariables, a `[command]` still expands any $variable inside it, since evaluating the command requires it; with -nocommands, a `$name([b])` array reference still runs the `[b]` inside its index, since resolving the variable requires it.\n\nTcl 9.1 adds inverted, positive-only switches -backslashes/-commands/-variables: giving any one of these turns every substitution off by default and enables only the ones named, so `subst -variables $s` is equivalent to `subst -nobackslashes -nocommands $s`. The two switch families cannot be combined in one call.\n\nA break inside an embedded command or variable index stops substitution immediately, keeping only the text produced so far; continue substitutes an empty string for that one embedded piece and keeps going; return (or any other completion code) substitutes the returned value for it. Any other error propagates as subst's own error.\n\n**Security**: without -nocommands (or a -variables/-backslashes-only call), any `[cmd]` in string is executed as Tcl, making subst a code-injection sink for untrusted input. Use `subst -nocommands $template` when only variable interpolation is needed, or prefer `[string map]` / `[format]` for safe templating.",
            source: "Tcl subst(n)",
            examples: "set name Alice\nputs [subst {Hello, $name! Today is [clock format [clock seconds] -format %A].}]\n\n# Safe templating: disable command substitution for untrusted input\nset template {Welcome, $user!}\nputs [subst -nocommands $template]\n\n# Tcl 9.1: enable only variable substitution\nputs [subst -variables {$name < [expr {1+1}] > \\n}]",
            return_value: "The fully-substituted string: break truncates it at the point of the exception, continue/return substitute their value for just that one embedded command or variable index, and any other error becomes subst's own error.",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        taint_sink_gate: Some(subst_evaluates_commands),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::{fold_subst, subst_evaluates_commands};

    #[test]
    fn subst_evaluates_commands_default_is_true() {
        assert!(subst_evaluates_commands(&["$tainted"]));
    }

    #[test]
    fn subst_evaluates_commands_legacy_nocommands_disables() {
        assert!(!subst_evaluates_commands(&["-nocommands", "$tainted"]));
        // Order-independent, and other legacy negatives don't interfere.
        assert!(!subst_evaluates_commands(&[
            "-novariables",
            "-nocommands",
            "$tainted"
        ]));
    }

    #[test]
    fn subst_evaluates_commands_legacy_other_negatives_leave_commands_on() {
        assert!(subst_evaluates_commands(&["-novariables", "$tainted"]));
        assert!(subst_evaluates_commands(&["-nobackslashes", "$tainted"]));
    }

    #[test]
    fn subst_evaluates_commands_positive_form_requires_explicit_commands() {
        // Tcl 9.1 positive form: everything defaults off, so
        // `-variables` alone never runs command substitution.
        assert!(!subst_evaluates_commands(&["-variables", "$tainted"]));
        assert!(!subst_evaluates_commands(&["-backslashes", "$tainted"]));
        assert!(subst_evaluates_commands(&["-commands", "$tainted"]));
        assert!(subst_evaluates_commands(&[
            "-variables",
            "-commands",
            "$tainted"
        ]));
    }

    #[test]
    fn subst_evaluates_commands_stops_scanning_at_the_string_operand() {
        // Only flags *before* the string operand matter — a `-nocommands`-
        // shaped word after it (an arity-error call shape) is the string
        // argument's own trailing content, not a switch, and must not
        // suppress the sink.
        assert!(subst_evaluates_commands(&["$x", "-nocommands"]));
    }

    #[test]
    fn subst_folds_only_substitution_free_strings() {
        // A plain `subst string` is its own
        // result; anything with $ / [ / \ bails (sound subset).
        assert_eq!(fold_subst(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(fold_subst(&["a b c"]).as_deref(), Some("a b c"));
        assert_eq!(fold_subst(&["$x"]), None, "variable substitution bails");
        assert_eq!(fold_subst(&["[cmd]"]), None, "command substitution bails");
        assert_eq!(fold_subst(&["a\\nb"]), None, "backslash bails");
        assert_eq!(
            fold_subst(&["-novariables", "x"]),
            None,
            "option form bails"
        );
    }
}
