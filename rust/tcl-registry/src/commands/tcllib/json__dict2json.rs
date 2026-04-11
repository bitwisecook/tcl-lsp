//! `json::dict2json` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::dict2json",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Convert a Tcl dict to a JSON string.",
            &["json::dict2json dictValue"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
