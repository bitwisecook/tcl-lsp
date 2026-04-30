//! `MESSAGE::proto` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::proto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns protocol of the message.",
            &["MESSAGE::proto"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
