//! `AUTH::ssl_cc_ldap_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::ssl_cc_ldap_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the status from the last successful client certificate-based LDAP query.",
            synopsis: &["AUTH::ssl_cc_ldap_status AUTH_ID"],
            snippet: "Returns the status from the last successful client certificate-based\nLDAP query for the specified authorization session <authid>. The system\nreturns an empty string if the last successful query did not perform a\nclient certificate-based LDAP query, or if no query has yet been\nperformed. This command has been deprecated in favor of\nAUTH::response_data.\n\nAUTH::ssl_cc_ldap_status <authid>\n\n     * Returns the status from the last successful client\n       certificate-based LDAP query for the specified authorization\n       session <authid>.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__ssl_cc_ldap_status.html",
            examples: "when RULE_INIT {\n    set tmm_auth_subscription \"*\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::ssl_cc_ldap_status AUTH_ID" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
