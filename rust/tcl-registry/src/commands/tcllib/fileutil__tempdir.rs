//! `fileutil::tempdir` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempdir",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Return the path of the system temporary directory.",
            &["fileutil::tempdir ?path?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
