//! `write_sdc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_sdc",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write SDC constraints to a file.",
            &["write_sdc ?-nosplit? ?-version version? ?file_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
