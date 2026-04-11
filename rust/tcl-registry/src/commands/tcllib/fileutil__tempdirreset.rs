//! `fileutil::tempdirReset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempdirReset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Reset the cached temporary directory path.",
            &["fileutil::tempdirReset"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
