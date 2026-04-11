//! `json::string2json` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::string2json",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Convert a Tcl string to a JSON string value.",
            &["json::string2json string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
