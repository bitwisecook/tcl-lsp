//! `STREAM::max_matchsize` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::max_matchsize",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets a maximum number of bytes that the system can buffer during partial matches",
            &["STREAM::max_matchsize SIZE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
