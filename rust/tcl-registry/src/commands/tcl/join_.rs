//! `join` — join list elements into a string.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "join",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Join list elements into a string.",
            &["join list ?joinString?"],
            "Tcl join(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
