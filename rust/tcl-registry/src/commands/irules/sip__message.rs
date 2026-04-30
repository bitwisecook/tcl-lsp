//! `SIP::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the full content of the SIP request or response message.",
            &["SIP::message"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
