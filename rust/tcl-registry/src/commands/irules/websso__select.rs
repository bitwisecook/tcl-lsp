//! `WEBSSO::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Use specified SSO configuration object to do SSO for the HTTP request.",
            synopsis: &["WEBSSO::select WEBSSO_OBJECT"],
            snippet: "This command causes APM to use specified SSO configuration object to do\nSSO for the HTTP request. Admin should make sure that the selected SSO\nmethod works for the specified request (and is enabled on backend\nserver request is going to). The scope of this iRule command is per\nHTTP request. Admin needs to execute it for each HTTP request.",
            source: "https://clouddocs.f5.com/api/irules/WEBSSO__select.html",
            examples: "when ACCESS_ACL_ALLOWED {\n    set req_uri [HTTP::uri]\n    if { $req_uri starts_with \"/owa\" } {\n        if { $req_uri eq \"/owa/auth/logon.aspx?url=https://mysite.com/owa/&reason=0\" } {\n            WEBSSO::select owa_form_base_sso\n        } elseif { $req_uri eq \"/owa/auth/logon.aspx?url=https://mysite.com/ecp/&reason=0\" } {\n            WEBSSO::select ecp_form_base_sso\n        }\n    }\n    unset req_uri\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "WEBSSO::select WEBSSO_OBJECT" },
        ],
        ..CommandSpec::DEFAULT
    }
}
