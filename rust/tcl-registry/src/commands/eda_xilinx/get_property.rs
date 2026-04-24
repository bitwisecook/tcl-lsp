//! `get_property` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_property",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Get a property from a Vivado design object.",
            &["get_property property_name object"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
