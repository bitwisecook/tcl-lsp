//! `fileutil::cat` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::cat",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Return the contents of one or more files.",
            &["fileutil::cat ?-encoding enc? ?--? file ..."],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
