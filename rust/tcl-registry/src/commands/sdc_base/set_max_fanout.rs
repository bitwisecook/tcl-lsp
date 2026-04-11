//! `set_max_fanout` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_max_fanout",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set maximum fanout constraint.",
            &["set_max_fanout fanout_value object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
