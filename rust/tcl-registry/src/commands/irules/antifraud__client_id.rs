//! `ANTIFRAUD::client_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::client_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns client id collected on client side.",
            &["ANTIFRAUD::client_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
