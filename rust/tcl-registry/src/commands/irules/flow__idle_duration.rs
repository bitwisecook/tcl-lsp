//! `FLOW::idle_duration` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::idle_duration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the time in seconds when the flow was last used.",
            &["FLOW::idle_duration ANY_CHARS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
