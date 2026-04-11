//! `ADAPT::context_current` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_current",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the current context.",
            &["ADAPT::context_current"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
