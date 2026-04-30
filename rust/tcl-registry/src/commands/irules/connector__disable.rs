//! `CONNECTOR::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable all the connectors on chain.",
            &["CONNECTOR::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
