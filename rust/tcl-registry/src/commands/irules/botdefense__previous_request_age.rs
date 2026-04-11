//! `BOTDEFENSE::previous_request_age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::previous_request_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the number of seconds that passed since the previous request was receive",
            &["BOTDEFENSE::previous_request_age"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
