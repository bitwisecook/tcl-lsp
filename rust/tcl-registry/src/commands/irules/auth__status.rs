//! `AUTH::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns authentication status.",
            &["AUTH::status (AUTH_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
