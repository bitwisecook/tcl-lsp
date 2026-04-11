//! `AUTH::wantcredential_prompt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_prompt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a string for an authorization session authidXs credential prompt.",
            &["AUTH::wantcredential_prompt AUTH_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
