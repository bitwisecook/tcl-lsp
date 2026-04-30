//! `STREAM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables the stream filter for the life of the current TCP connection or until di",
            &["STREAM::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
