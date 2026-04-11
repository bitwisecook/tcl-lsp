//! `set_max_delay` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_max_delay",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set maximum delay constraint on a path.", &["set_max_delay ?-from from_list? ?-through through_list? ?-to to_list? ?-rise | -fall? delay_value"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
