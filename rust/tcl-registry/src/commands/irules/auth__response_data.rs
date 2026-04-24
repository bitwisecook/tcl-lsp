//! `AUTH::response_data` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::response_data",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns pairwise auth query results.",
            &["AUTH::response_data (AUTH_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
