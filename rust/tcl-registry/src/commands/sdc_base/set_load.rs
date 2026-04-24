//! `set_load` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_load",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set capacitive load on nets or ports.", &["set_load ?-pin_load | -wire_load? ?-min | -max? ?-subtract_pin_load? value objects"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
