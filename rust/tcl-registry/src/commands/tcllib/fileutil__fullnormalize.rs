//! `fileutil::fullnormalize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::fullnormalize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Fully normalise a path (resolves symlinks).",
            &["fileutil::fullnormalize path"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
