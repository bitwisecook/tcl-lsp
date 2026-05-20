//! `apply` — apply an anonymous procedure.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Apply an anonymous procedure.",
            &["apply func ?arg ...?"],
            "Tcl apply(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
