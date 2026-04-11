//! `AUTH::ssl_cc_ldap_username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::ssl_cc_ldap_username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a user name that the system retrieved from the LDAP database.",
            &["AUTH::ssl_cc_ldap_username AUTH_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
