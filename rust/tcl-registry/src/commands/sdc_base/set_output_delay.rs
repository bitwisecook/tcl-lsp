//! `set_output_delay` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_output_delay",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set output delay on ports.", &["set_output_delay ?-clock clock_name? ?-clock_fall? ?-level_sensitive? ?-rise | -fall? ?-min | -max? "], "F5")),
        ..CommandSpec::DEFAULT
    }
}
