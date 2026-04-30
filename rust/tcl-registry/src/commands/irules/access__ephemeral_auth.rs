//! `ACCESS::ephemeral-auth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::ephemeral-auth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Ephemeral auth related iRule", &["ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
