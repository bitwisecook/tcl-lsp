//! `set_max_capacitance` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_max_capacitance",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set maximum capacitance constraint.",
            &["set_max_capacitance cap_value object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
