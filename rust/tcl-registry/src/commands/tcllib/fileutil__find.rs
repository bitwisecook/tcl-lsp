//! `fileutil::find` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::find",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Recursively find files matching a filter.",
            &["fileutil::find ?basedir? ?filtercmd?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
