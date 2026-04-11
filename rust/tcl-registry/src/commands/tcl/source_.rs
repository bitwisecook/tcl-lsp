//! `source` — evaluate a file or resource as a Tcl script.

use crate::prelude::*;

/// Command spec for `source`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "source",
        traits: Traits::LANGUAGE_KEYWORD | Traits::SOURCES_FILE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[OptionSpec {
            name: "-encoding",
            takes_value: true,
            value_hint: "encoding",
            detail: "",
        }],
        hover: Some(HoverSnippet::brief(
            "Evaluate a file or resource as a Tcl script.",
            &["source ?-encoding name? fileName"],
            "Tcl source(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
