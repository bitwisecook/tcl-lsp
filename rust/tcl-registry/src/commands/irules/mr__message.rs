//! `MR::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets details in the message routing table.",
            &["MR::message clone (CLONE_ID)+"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
