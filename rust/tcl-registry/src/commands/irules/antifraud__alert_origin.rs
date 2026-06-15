//! `ANTIFRAUD::alert_origin` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_origin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the origin of the alert, e.g.",
            synopsis: &["ANTIFRAUD::alert_origin (VALUE)?"],
            snippet: "ANTIFRAUD::alert_origin ;\n                Returns the origin of the alert, e.g. clientside, serverside or secure alert cookie.\n\n            ANTIFRAUD::alert_origin VALUE ;\n                Sets the origin of the alert, e.g. clientside, serverside or secure alert cookie.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_origin.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert origin: [ANTIFRAUD::alert_origin].\"\n                ANTIFRAUD::alert_origin new_value\n                log local0. \"new Alert origin: [ANTIFRAUD::alert_origin].\"\n            }",
            return_value: "ANTIFRAUD::alert_origin ; Returns the origin of the alert, e.g. clientside, serverside or secure alert cookie.",
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
            synopsis: "ANTIFRAUD::alert_origin (VALUE)?",
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
