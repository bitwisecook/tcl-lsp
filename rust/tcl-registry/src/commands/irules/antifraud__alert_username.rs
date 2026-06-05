//! `ANTIFRAUD::alert_username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets username and for phishing also additional fields.",
            synopsis: &["ANTIFRAUD::alert_username (VALUE)?"],
            snippet: "ANTIFRAUD::alert_username ;\n                Returns username and for phishing also additional fields.\n\n            ANTIFRAUD::alert_username VALUE ;\n                Sets username and for phishing also additional fields.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_username.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert username: [ANTIFRAUD::alert_username].\"\n                ANTIFRAUD::alert_username new_value\n                log local0. \"new Alert username: [ANTIFRAUD::alert_username].\"\n            }",
            return_value: "ANTIFRAUD::alert_username ; Returns username and for phishing also additional fields.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_username (VALUE)?" },
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
