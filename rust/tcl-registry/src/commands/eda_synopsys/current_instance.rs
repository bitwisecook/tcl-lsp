//! `current_instance` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "current_instance",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Navigate into a hierarchical instance.",
            &["current_instance ?instance_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
