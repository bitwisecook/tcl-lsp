//! `ACCESS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Control enforcement for a particular request URI.",
            &["ACCESS::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
