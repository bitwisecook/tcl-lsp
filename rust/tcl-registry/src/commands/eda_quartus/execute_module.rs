//! `execute_module` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "execute_module",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Execute a specific Quartus module.",
            &["execute_module -tool tool_name ?-args arg_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
