//! `HTTP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::release",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases the data collected via HTTP::collect.",
            &["HTTP::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
