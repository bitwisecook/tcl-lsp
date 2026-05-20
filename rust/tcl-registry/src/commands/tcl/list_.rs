//! `list` — create a Tcl list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "list",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Create a Tcl list.",
            &["list ?arg ...?"],
            "Tcl list(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
