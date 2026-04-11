//! `set_min_delay` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_min_delay",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set minimum delay constraint on a path.", &["set_min_delay ?-from from_list? ?-through through_list? ?-to to_list? ?-rise | -fall? delay_value"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
