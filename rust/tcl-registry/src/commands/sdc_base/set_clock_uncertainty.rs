//! `set_clock_uncertainty` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_uncertainty",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set clock uncertainty (jitter).", &["set_clock_uncertainty ?-setup | -hold? ?-from from_clock? ?-to to_clock? ?-rise | -fall? uncertainty"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
