//! `fileutil::tempfile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempfile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Create a temporary file and return its path.",
            &["fileutil::tempfile ?prefix?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
