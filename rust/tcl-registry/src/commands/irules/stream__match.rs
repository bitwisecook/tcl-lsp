//! `STREAM::match` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::match",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the matching characters.",
            &["STREAM::match"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
