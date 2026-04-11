//! `elaborate` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "elaborate",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Elaborate the design hierarchy.",
            &["elaborate ?design_name? ?-parameters params?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
