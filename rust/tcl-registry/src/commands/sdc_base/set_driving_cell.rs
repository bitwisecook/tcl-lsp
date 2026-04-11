//! `set_driving_cell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_driving_cell",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set the driving cell for input ports.", &["set_driving_cell ?-lib_cell cell_name? ?-library lib? ?-pin pin_name? ?-from_pin from_pin_name? ?-no"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
