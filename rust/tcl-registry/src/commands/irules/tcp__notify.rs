//! `TCP::notify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Sends a message to upper layers of iRule processing.",
            &["TCP::notify (request | response | eom)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
