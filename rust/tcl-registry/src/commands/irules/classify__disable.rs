//! `CLASSIFY::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the classification of the flow.",
            &["CLASSIFY::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
