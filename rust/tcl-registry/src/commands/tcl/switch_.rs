//! `switch` — pattern-based branching on a subject string.

use crate::prelude::*;

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
        if args[i + 1] != "-" {
            if let Ok(idx) = u8::try_from(i + 1) {
                roles.push((idx, ArgRole::Body));
            }
        }
        i += 2;
    }
    roles
}

/// Command spec for `switch`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "switch",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::HAS_SWITCH_BODY,
        arity: Arity::at_least(2),
        arg_role_resolver: Some(switch_arg_roles),
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-exact",
                takes_value: false,
                value_hint: "",
                detail: "Exact string compare mode.",
            },
            OptionSpec {
                name: "-glob",
                takes_value: false,
                value_hint: "",
                detail: "Glob pattern mode.",
            },
            OptionSpec {
                name: "-regexp",
                takes_value: false,
                value_hint: "",
                detail: "Regular expression mode.",
            },
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "Case-insensitive matching.",
            },
            OptionSpec {
                name: "-matchvar",
                takes_value: true,
                value_hint: "varName",
                detail: "Store match in variable (regexp mode).",
            },
            OptionSpec {
                name: "-indexvar",
                takes_value: true,
                value_hint: "varName",
                detail: "Store match indices in variable (regexp mode).",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "End of options.",
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Pattern-based branching on a subject string.",
            &[
                "switch ?options? string pattern body ?pattern body ...?",
                "switch ?options? string {pattern body ?pattern body ...?}",
            ],
            "Tcl switch(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
