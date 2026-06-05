//! `msgcat::mcflset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcflset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Set a translation for the locale of the current message file.",
            synopsis: &["msgcat::mcflset src-string ?translate-string?"],
            snippet: "",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
