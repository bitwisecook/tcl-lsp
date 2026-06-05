//! `msgcat::mcn` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcn",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Translate a source string in a given namespace.",
            synopsis: &["msgcat::mcn namespace src-string ?arg arg ...?"],
            snippet: "",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
