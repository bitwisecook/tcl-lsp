//! `LDAP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LDAP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable LDAP STARTTLS.",
            &["LDAP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
