//! `set_multicycle_path` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_multicycle_path",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Define a multicycle timing path.", &["set_multicycle_path ?-setup | -hold? ?-start | -end? ?-from from_list? ?-through through_list? ?-to "], "F5")),
        ..CommandSpec::DEFAULT
    }
}
