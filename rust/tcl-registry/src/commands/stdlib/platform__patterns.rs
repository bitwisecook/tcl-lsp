//! `platform::patterns` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::patterns",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return a list of platform patterns that match the given identifier.",
            synopsis: &["platform::patterns id"],
            snippet: "Given a platform *id* from ``platform::identify``, returns a list of platform patterns in order of decreasing specificity.",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform"),
        ..CommandSpec::DEFAULT
    }
}
