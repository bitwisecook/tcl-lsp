//! `set_max_transition` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_max_transition",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set maximum transition time constraint.",
            &["set_max_transition transition_value ?-clock_path? ?-data_path? object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
