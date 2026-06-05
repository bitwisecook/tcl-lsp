//! `puts` — write to a channel.

use crate::prelude::*;

/// Command spec for `puts`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "puts",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::TAINT_SINK,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[OptionSpec {
            name: "-nonewline",
            takes_value: false,
            value_hint: "",
            detail: "Do not output a newline character.",
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Write to a channel.",
            &["puts ?-nonewline? ?channelId? string"],
            "Tcl puts(1)",
        )),
        // GAP-D2: tainted data reaching `puts` output → T101.
        // Mirrors `tcl/puts_.py`.
        taint_output_sink: Some("T101"),
        ..CommandSpec::DEFAULT
    }
}
