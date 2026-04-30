//! `fconfigure` — set and get options on a channel.

use crate::prelude::*;

/// Command spec for `fconfigure`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fconfigure",
        traits: Traits::CONFIGURES_CHANNEL,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-blocking",
                takes_value: true,
                value_hint: "boolean",
                detail: "",
            },
            OptionSpec {
                name: "-buffering",
                takes_value: true,
                value_hint: "mode",
                detail: "",
            },
            OptionSpec {
                name: "-buffersize",
                takes_value: true,
                value_hint: "size",
                detail: "",
            },
            OptionSpec {
                name: "-encoding",
                takes_value: true,
                value_hint: "encoding",
                detail: "",
            },
            OptionSpec {
                name: "-eofchar",
                takes_value: true,
                value_hint: "chars",
                detail: "",
            },
            OptionSpec {
                name: "-translation",
                takes_value: true,
                value_hint: "mode",
                detail: "",
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Set and get options on a channel.",
            &["fconfigure channelId ?optionName? ?value ...?"],
            "Tcl fconfigure(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
