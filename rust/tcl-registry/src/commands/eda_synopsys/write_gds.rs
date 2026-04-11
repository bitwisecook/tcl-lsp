//! `write_gds` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_gds",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a GDSII stream file.",
            &["write_gds ?-long_names? ?-output file_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
