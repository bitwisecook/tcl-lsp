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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nobackslashes",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocommands",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-novariables",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // Tcl 9.1 (TIP) adds positive forms that enable *only* the named
    // substitution.  Positive and negated options may not be combined in one
    // call.
    OptionSpec {
        name: "-backslashes",
        value: OptionValue::flag(),
        detail: "Enable only backslash substitution (Tcl 9.1).",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-commands",
        value: OptionValue::flag(),
        detail: "Enable only command substitution (Tcl 9.1).",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-variables",
        value: OptionValue::flag(),
        detail: "Enable only variable substitution (Tcl 9.1).",
        dialects: Some(DialectSet::TCL91),
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "subst ?options? string",
}];

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
        traits: Traits::TAINT_SINK | Traits::IS_UNESCAPE | Traits::PERFORMS_SUBSTITUTION,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold: Some(fold_subst),
        hover: Some(HoverSnippet {
            summary: "Perform backslash, command, and variable substitutions.",
            synopsis: &[
                "subst ?options? string",
                "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
            ],
            snippet: "**Security**: Without `-nocommands`, any `[cmd]` in the string is executed as Tcl. Use `-nocommands` when only variable substitution is needed: `subst -nocommands $template`. For safe templating, prefer `[string map]` or `[format]`.",
            source: "Tcl subst(1)",
            examples: "",
            return_value: "The string with substitutions applied.",
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
