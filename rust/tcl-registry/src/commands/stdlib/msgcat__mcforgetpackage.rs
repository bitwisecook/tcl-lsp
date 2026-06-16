//! `msgcat::mcforgetpackage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcforgetpackage",
        dialects: None,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Remove all translations for the calling package.",
            synopsis: &["msgcat::mcforgetpackage"],
            snippet: "",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
