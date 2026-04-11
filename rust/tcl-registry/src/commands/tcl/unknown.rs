//! `unknown` — handle unknown commands.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unknown",
        traits: Traits::CREATES_BARRIER,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Handle unknown command names.",
            &["unknown cmdName ?arg ...?"],
            "Tcl unknown(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
