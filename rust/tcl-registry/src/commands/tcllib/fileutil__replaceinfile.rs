//! `fileutil::replaceInFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::replaceInFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(4),
        hover: Some(HoverSnippet::brief(
            "Replace data in a file.",
            &["fileutil::replaceInFile ?options? file at n data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
