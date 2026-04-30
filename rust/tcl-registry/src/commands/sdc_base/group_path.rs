//! `group_path` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "group_path",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Group paths for multi-scenario analysis.", &["group_path ?-name group_name? ?-from from_list? ?-through through_list? ?-to to_list? ?-weight weigh"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
