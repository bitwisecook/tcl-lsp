//! `AUTH::ssl_cc_ldap_username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::ssl_cc_ldap_username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a user name that the system retrieved from the LDAP database.",
            synopsis: &["AUTH::ssl_cc_ldap_username AUTH_ID"],
            snippet: "Returns the user name that the system retrieved from the LDAP database\nfrom the last successful client certificate-based LDAP query for the\nspecified authorization session <authid>. The system returns an empty\nstring if the last successful query did not perform a successful client\ncertificate-based LDAP query, or if no query has yet been performed.\nThis command has been deprecated in favor of AUTH::response_data.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__ssl_cc_ldap_username.html",
            examples: "when RULE_INIT {\n    set cc_ldap_username \"defaultuser\"\n    set tmm_auth_subscription \"*\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::ssl_cc_ldap_username AUTH_ID",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
