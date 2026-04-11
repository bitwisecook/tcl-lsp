//! `STREAM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the stream filter for a connection.",
            &["STREAM::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
