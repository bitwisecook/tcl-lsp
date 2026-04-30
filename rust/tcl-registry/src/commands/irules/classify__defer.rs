//! `CLASSIFY::defer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::defer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Defers the classification of the flow to response.",
            &["CLASSIFY::defer"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
