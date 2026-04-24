//! `fileutil::updateInPlace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::updateInPlace",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Update a file in place using a command.",
            &["fileutil::updateInPlace ?options? fileName cmdOrBody"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
