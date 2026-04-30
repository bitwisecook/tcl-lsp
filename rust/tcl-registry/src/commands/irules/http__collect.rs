//! `HTTP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::collect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collects an amount of HTTP body data that you specify.",
            &["HTTP::collect (CONTENT_LENGTH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
