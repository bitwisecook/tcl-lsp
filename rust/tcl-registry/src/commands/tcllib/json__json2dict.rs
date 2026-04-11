//! `json::json2dict` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::json2dict",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Convert a JSON string to a Tcl dict.",
            &["json::json2dict jsonText"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
