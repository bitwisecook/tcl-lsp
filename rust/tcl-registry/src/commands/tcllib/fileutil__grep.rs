//! `fileutil::grep` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::grep",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Search for a pattern in files.",
            &["fileutil::grep pattern ?files?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
