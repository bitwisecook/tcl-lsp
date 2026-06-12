//! `ASM::login_status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::login_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Request status of the login session tracked by one of the login pages defined in the policy.",
            synopsis: &["ASM::login_status"],
            snippet: "Returns status of the login session tracked by one of the login pages defined in the policy. Following are the possible values:\n\n                not_logged_in: The request is not within a login session.\n                logging_in: The request is to a login URL.\n                logged_in: The request is within a login session, indicates a successful login in the ASM_RESPONSE_LOGIN event.\n                failed: The login attempt is failed, triggered only in the ASM_RESPONSE_LOGIN event.",
            source: "https://clouddocs.f5.com/api/irules/ASM__login_status.html",
            examples: "when ASM_RESPONSE_LOGIN {\n                if {[ASM::login_status] eq \"logged_in\"} {\n                    log local0. \"User [ASM::username] logged in succesfully.\"\n                }\n                else {\n                    log local0. \"Login attempt to [ASM::username] failed.\"\n                }\n            }",
            return_value: "Returns status of the login session.;",
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
            FormSpec { kind: FormKind::Default, synopsis: "ASM::login_status" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
