//! `lsearch` — search a list for a pattern.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsearch",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Search a list for a pattern.",
            &["lsearch ?option ...? list pattern"],
            "Tcl lsearch(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
