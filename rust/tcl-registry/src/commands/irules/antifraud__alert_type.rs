//! `ANTIFRAUD::alert_type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets alert type.",
            synopsis: &["ANTIFRAUD::alert_type (VALUE)?"],
            snippet: "ANTIFRAUD::alert_type ;\n                Returns alert type.\n\n            ANTIFRAUD::alert_type VALUE ;\n                Sets alert type.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_type.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert type: [ANTIFRAUD::alert_type].\"\n                ANTIFRAUD::alert_type new_value\n                log local0. \"new Alert type: [ANTIFRAUD::alert_type].\"\n            }",
            return_value: "ANTIFRAUD::alert_type ; Returns alert type.",
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
            synopsis: "ANTIFRAUD::alert_type (VALUE)?",
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
