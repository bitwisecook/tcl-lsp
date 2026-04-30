//! `fileutil::removeFromFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::removeFromFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Remove data from a file.",
            &["fileutil::removeFromFile ?options? file at n"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
