//! `HTTP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::release",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases the data collected via HTTP::collect.",
            synopsis: &["HTTP::release"],
            snippet: "Releases the data collected via HTTP::collect. Unless a subsequent\nHTTP::collect command was issued, there is no need to use the\nHTTP::release command inside of the HTTP_REQUEST_DATA and\nHTTP_RESPONSE_DATA events, since (in these cases) the data is\nimplicitly released.\nIt is important to note that these semantics are different than those\nof the TCP::collect and TCP::release commands.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__release.html",
            examples: "when CLIENT_ACCEPTED {\n    set tmm_auth_ldap_sid [AUTH::start pam default_ldap]\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::release" },
        ],
        ..CommandSpec::DEFAULT
    }
}
