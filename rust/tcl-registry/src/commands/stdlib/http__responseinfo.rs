//! `http::responseInfo` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseInfo",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        return_type: Some(TclType::Dict),
        hover: Some(HoverSnippet {
            summary: "Return a dict of response metadata.",
            synopsis: &["http::responseInfo token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
