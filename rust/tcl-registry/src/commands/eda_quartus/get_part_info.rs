//! `get_part_info` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_part_info",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get information about a specific FPGA part.", &["get_part_info -family | -speed_grade | -package | -pin_count | -available_pin_count part_name"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
