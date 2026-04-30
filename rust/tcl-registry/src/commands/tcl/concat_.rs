//! `concat` — concatenate lists.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "concat",
        traits: Traits::PURE | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Join lists into a single list.",
            &["concat ?arg ...?"],
            "Tcl concat(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
