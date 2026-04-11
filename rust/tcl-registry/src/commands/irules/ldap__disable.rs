//! `LDAP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LDAP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable LDAP STARTTLS.",
            &["LDAP::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
