//! `regexp` — match a regular expression against a string.

use crate::prelude::*;

/// Command spec for `regexp`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp",
        traits: Traits::WARN_WITHOUT_TERMINATOR,
        arity: Arity::at_least(1),
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-expanded",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-line",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-linestop",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-lineanchor",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-all",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-inline",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-indices",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-start",
                takes_value: true,
                value_hint: "index",
                detail: "",
            },
            OptionSpec {
                name: "-about",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Match a regular expression against a string.",
            &["regexp ?switches? exp string ?matchVar? ?subMatchVar ...?"],
            "Tcl regexp(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
