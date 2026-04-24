//! `DIAMETER::is_response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns true if it is a DIAMETER response, otherwise, returns false.",
            &["DIAMETER::is_response"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
