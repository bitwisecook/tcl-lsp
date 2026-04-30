//! `glob` — return names of files that match patterns.

use crate::prelude::*;

/// Command spec for `glob`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "glob",
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-directory",
                takes_value: true,
                value_hint: "dir",
                detail: "",
            },
            OptionSpec {
                name: "-join",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-nocomplain",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-path",
                takes_value: true,
                value_hint: "pathPrefix",
                detail: "",
            },
            OptionSpec {
                name: "-tails",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "-types",
                takes_value: true,
                value_hint: "typeList",
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
            "Return names of files that match patterns.",
            &["glob ?switches? ?--? pattern ?pattern ...?"],
            "Tcl glob(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
