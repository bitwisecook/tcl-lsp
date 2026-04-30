//! `read_sdc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_sdc",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Read SDC constraint file.",
            &["read_sdc ?-echo? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
