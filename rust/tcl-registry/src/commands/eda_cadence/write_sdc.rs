//! `write_sdc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_sdc",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write SDC constraints.",
            &["write_sdc > file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
