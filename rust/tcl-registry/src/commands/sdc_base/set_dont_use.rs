//! `set_dont_use` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_dont_use",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Prevent use of library cells.",
            &["set_dont_use lib_cell_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
