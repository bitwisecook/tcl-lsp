//! `ACCESS::saml` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::saml",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Access or manipulate SAML related messages.",
            &["ACCESS::saml authn (CONTENT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
