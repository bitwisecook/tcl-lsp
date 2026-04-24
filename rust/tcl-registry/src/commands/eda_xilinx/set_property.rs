//! `set_property` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_property",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Set a property on a Vivado design object.",
            &["set_property property_name value objects"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
