//! `puts` — write to a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "puts ?-nonewline? ?channelId? string",
}];

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
        hover: Some(HoverSnippet {
            summary: "Write text to a channel (stdout by default).",
            synopsis: &["puts ?-nonewline? ?channelId? string"],
            snippet: "Use `-nonewline` to suppress the trailing newline.",
            source: "Tcl puts(1)",
            examples: "",
            return_value: "",
        }),
        // Tainted data reaching `puts` output → T101.
        taint_output_sink: Some("T101"),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
