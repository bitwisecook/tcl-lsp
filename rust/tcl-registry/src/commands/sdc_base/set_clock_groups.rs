//! `set_clock_groups` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_groups",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set mutual exclusivity between clock groups.", &["set_clock_groups ?-physically_exclusive | -logically_exclusive | -asynchronous? -group clock_list .."], "F5")),
        ..CommandSpec::DEFAULT
    }
}
