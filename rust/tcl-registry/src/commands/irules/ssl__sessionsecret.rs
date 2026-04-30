//! `SSL::sessionsecret` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionsecret",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return data about SSL handshake master secret.",
            &["SSL::sessionsecret"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
