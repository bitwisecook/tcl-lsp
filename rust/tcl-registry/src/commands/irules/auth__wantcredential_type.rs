//! `AUTH::wantcredential_type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an authorization session authidXs credential type.",
            &["AUTH::wantcredential_type AUTH_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
