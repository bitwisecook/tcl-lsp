//! `MESSAGE::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the type of the current message.",
            &["MESSAGE::type"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
