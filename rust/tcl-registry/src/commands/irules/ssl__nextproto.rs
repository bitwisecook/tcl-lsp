//! `SSL::nextproto` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::nextproto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the Next Protocol Negotiation (NPN) string.",
            &["SSL::nextproto"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
