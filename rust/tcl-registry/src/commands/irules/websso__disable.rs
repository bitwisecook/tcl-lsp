//! `WEBSSO::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forwards a request without doing SSO processing on it.",
            &["WEBSSO::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
