//! `write_def` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_def",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a DEF file.",
            &["write_def ?-version version? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
