//! `json::list2json` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::list2json",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Convert a Tcl list to a JSON array.",
            &["json::list2json list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
