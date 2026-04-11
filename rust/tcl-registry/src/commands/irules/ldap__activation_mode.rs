//! `LDAP::activation_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LDAP::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the activation mode.",
            &["LDAP::activation_mode (none | allow | require)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
