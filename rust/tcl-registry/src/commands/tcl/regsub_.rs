//! `regsub` — perform substitutions based on regular expression matching.

use crate::prelude::*;

/// Command spec for `regsub`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regsub",
        arity: Arity::new(3, 4),
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
                dialects: None,
            },
            OptionSpec {
                name: "-expanded",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-line",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-linestop",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-lineanchor",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-all",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-start",
                takes_value: true,
                value_hint: "index",
                detail: "",
                dialects: None,
            },
            // `regsub -command` is Tcl 9.0+ (TIP 463).
            OptionSpec {
                name: "-command",
                takes_value: false,
                value_hint: "",
                detail: "Treat subSpec as a command prefix to call per match.",
                dialects: Some(DialectSet::TCL90),
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Perform substitutions based on regular expression matching.",
            &["regsub ?switches? exp string subSpec ?varName?"],
            "Tcl regsub(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
