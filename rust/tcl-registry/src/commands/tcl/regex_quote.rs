//! Regex quoting helper aliases.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for regex.",
            &["regex_quote string"],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
