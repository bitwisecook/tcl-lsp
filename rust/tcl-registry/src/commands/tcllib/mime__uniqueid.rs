//! `mime::uniqueID` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::uniqueID",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Generate a globally unique identifier.",
            &["mime::uniqueID"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
