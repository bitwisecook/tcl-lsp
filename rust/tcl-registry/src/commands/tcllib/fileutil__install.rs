//! `fileutil::install` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::install",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Copy a file and optionally set permissions.",
            &["fileutil::install ?-m mode? source destination"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
