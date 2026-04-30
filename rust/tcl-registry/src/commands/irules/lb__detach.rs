//! `LB::detach` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::detach",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Detaches the server-side connection from the client-side.",
            &["LB::detach ('disable')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
