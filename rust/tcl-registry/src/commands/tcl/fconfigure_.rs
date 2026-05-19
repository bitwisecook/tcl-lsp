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
                dialects: None,
            },
            OptionSpec {
                name: "-buffering",
                takes_value: true,
                value_hint: "mode",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-buffersize",
                takes_value: true,
                value_hint: "size",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-encoding",
                takes_value: true,
                value_hint: "encoding",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-eofchar",
                takes_value: true,
                value_hint: "chars",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-translation",
                takes_value: true,
                value_hint: "mode",
                detail: "",
                dialects: None,
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
