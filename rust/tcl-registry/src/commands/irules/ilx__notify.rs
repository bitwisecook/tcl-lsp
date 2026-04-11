//! `ILX::notify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Calls an ILX method asynchronously.",
            &["ILX::notify HANDLE METHOD (ARGS)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
