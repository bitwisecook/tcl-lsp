//! `GTP::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the entire GTP message.",
            &["GTP::message ('-message' MESSAGE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
