//! `AUTH::authenticate_continue` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::authenticate_continue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Continues an authentication operation.",
            &["AUTH::authenticate_continue AUTH_ID RESPONSE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
