//! Regex quoting helper aliases.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for regexp.",
            &["regexp_quote string"],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
