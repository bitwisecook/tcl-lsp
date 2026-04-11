//! `set_technology` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_technology",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the target technology.",
            &["set_technology ?technology?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
