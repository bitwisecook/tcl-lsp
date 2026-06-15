//! `ASM::username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "request username from a login attempt throughout the login session.",
            synopsis: &["ASM::username"],
            snippet: "Returns the username from a login attempt throughout the login session. If there is no login session or the login page in the policy does not extract credentials, then an empty string is returned.;",
            source: "https://clouddocs.f5.com/api/irules/ASM__username.html",
            examples: "when ASM_REQUEST_DONE {\n                if {[ASM::is_authenticated]} {\n                    log local0. \"This request was sent by user [ASM::username].\"\n                }\n            }",
            return_value: "returns the username of an active login session or a login request.;",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::username",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
