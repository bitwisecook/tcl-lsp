//! `lindex` — retrieve an element from a list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lindex",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Retrieve an element from a list.",
            &["lindex list ?index ...?"],
            "Tcl lindex(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
