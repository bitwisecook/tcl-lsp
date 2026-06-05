//! `ASM::is_authenticated` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::is_authenticated",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Request login status of the user in the present request.",
            synopsis: &["ASM::is_authenticated"],
            snippet: "Returns true, if the user in the present request is logged in, that is, the user is authenticated successfully in one of the login pages defined in the policy and the session has not expired. This is synonymous to `[ASM::login_status] eq \"logged_in\"`.;",
            source: "https://clouddocs.f5.com/api/irules/ASM__is_authenticated.html",
            examples: "when ASM_REQUEST_DONE {\n                if {[ASM::is_authenticated]} {\n                    log local0. \"This request was sent by user [ASM::username].\"\n                }\n            }",
            return_value: "Returns true user in the current request is logged in.;",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::is_authenticated" },
        ],
        ..CommandSpec::DEFAULT
    }
}
