//! `platform::generic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::generic",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return the generic platform identifier (less specific than identify).",
            synopsis: &["platform::generic"],
            snippet: "",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform"),
        ..CommandSpec::DEFAULT
    }
}
