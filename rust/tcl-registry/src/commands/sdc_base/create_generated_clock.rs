//! `create_generated_clock` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_generated_clock",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Create a generated clock object.", &["create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-mult"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
