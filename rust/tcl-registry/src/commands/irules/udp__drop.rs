//! `UDP::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Drops the current UDP packet without removing the flow from the connection table",
            &["UDP::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
