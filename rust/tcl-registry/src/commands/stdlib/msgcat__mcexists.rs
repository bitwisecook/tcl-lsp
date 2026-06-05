//! `msgcat::mcexists` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcexists",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Check whether a translation exists for the given source string.",
            synopsis: &["msgcat::mcexists ?-exactnamespace? ?-exactlocale? src-string"],
            snippet: "",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
