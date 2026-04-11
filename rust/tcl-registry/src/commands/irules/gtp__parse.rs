//! `GTP::parse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a new GTP message from a byte stream.",
            &["GTP::parse BYTE_STREAM"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
