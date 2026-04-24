//! `LB::enable_decisionlog` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::enable_decisionlog",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables LTM decision logging",
            &["LB::enable_decisionlog"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
