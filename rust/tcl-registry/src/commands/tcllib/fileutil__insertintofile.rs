//! `fileutil::insertIntoFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::insertIntoFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Insert data into a file at an offset.",
            &["fileutil::insertIntoFile ?options? file at data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
