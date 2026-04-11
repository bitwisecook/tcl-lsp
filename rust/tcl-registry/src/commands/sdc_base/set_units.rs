//! `set_units` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_units",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set the unit specifications.", &["set_units ?-time unit? ?-capacitance unit? ?-resistance unit? ?-voltage unit? ?-current unit? ?-powe"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
