//! `fileutil::test` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::test",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Test file properties.",
            &["fileutil::test path codes ?msgvar? ?label?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
