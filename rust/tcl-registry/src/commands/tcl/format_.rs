//! `format` — format a string using printf-style conversion.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "format",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Format a string.",
            &["format formatString ?arg ...?"],
            "Tcl format(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
