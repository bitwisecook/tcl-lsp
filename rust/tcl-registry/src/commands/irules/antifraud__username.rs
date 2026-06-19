//! `ANTIFRAUD::username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets username value, only in context of ANTIFRAUD_LOGIN event.",
            synopsis: &["ANTIFRAUD::username (USERNAME_ALIAS)?"],
            snippet: "ANTIFRAUD::username\n                Returns username value, only in context of ANTIFRAUD_LOGIN event.\n\n            ANTIFRAUD::username USERNAME_ALIAS ;\n                Replaces existing username with USERNAME_ALIAS, only in context of ANTIFRAUD_LOGIN event.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__username.html",
            examples: "when ANTIFRAUD_LOGIN {\n                log local0. \"Username [ANTIFRAUD::username] tried to log in.\"\n                ANTIFRAUD::username username_alias\n                log local0. \"Username alias is [ANTIFRAUD::username].\"\n            }",
            return_value: "ANTIFRAUD::username Returns username value, only in context of ANTIFRAUD_LOGIN event.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::username (USERNAME_ALIAS)?",
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
