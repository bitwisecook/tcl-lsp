//! `BOTDEFENSE::previous_support_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::previous_support_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the Device ID of the client, as retrieved from the request.",
            &["BOTDEFENSE::previous_support_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
