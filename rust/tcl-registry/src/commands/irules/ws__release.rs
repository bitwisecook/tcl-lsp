//! `WS::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases the data collected using WS::collect.",
            &["WS::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
