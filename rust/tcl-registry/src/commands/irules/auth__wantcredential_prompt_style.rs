//! `AUTH::wantcredential_prompt_style` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_prompt_style",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an authorization session authidXs credential prompt style.",
            &["AUTH::wantcredential_prompt_style AUTH_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
