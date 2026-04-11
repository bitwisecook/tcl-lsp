//! `GTP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the entire payload for G-PDU message.",
            &["GTP::payload"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
