//! `fileutil::fileType` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::fileType",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Determine the type of a file.",
            &["fileutil::fileType filename"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
