//! `set_max_area` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_max_area",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set maximum area constraint.",
            &["set_max_area area_value"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
