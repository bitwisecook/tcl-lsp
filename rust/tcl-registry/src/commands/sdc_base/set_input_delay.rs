//! `set_input_delay` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_input_delay",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set input delay on ports.", &["set_input_delay ?-clock clock_name? ?-clock_fall? ?-level_sensitive? ?-rise | -fall? ?-min | -max? ?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
