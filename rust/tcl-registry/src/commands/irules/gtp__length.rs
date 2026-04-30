//! `GTP::length` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This value is returned as read from the message header.",
            &["GTP::length ('-message' MESSAGE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
