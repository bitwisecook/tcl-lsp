//! `read_ip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read IP core files (.xci).",
            &["read_ip ?-quiet? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
