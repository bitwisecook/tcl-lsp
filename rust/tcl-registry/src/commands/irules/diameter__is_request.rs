//! `DIAMETER::is_request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns true if the current message is a DIAMETER request.",
            &["DIAMETER::is_request"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
