//! `GENERICMESSAGE::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets values for messages in the generic message profile.",
            &["GENERICMESSAGE::message (len | length)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
