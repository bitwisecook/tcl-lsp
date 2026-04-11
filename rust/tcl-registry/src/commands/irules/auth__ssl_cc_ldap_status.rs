//! `AUTH::ssl_cc_ldap_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::ssl_cc_ldap_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the status from the last successful client certificate-based LDAP query.",
            &["AUTH::ssl_cc_ldap_status AUTH_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
