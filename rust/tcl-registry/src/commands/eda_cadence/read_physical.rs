//! `read_physical` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_physical",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read physical data (LEF/DEF).",
            &["read_physical ?-lef file_list? ?-def file?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
