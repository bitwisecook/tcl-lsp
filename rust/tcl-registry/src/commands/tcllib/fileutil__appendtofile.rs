//! `fileutil::appendToFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::appendToFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Append data to a file.",
            &["fileutil::appendToFile ?options? file data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
