//! `MR::store` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::store",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Stores a tcl variable with the mr_message object.",
            &["MR::store (VAR)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
