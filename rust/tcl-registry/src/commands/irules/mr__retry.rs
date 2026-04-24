//! `MR::retry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::retry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Send the current message to the router for routing.",
            &["MR::retry"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
