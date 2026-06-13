//! `ANTIFRAUD::alert_component` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_component",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets error type according to alert_type.",
            synopsis: &["ANTIFRAUD::alert_component (VALUE)?"],
            snippet: "ANTIFRAUD::alert_component ;\n                Returns error type according to alert_type.\n\n            ANTIFRAUD::alert_component VALUE ;\n                Sets error type according to alert_type.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_component.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert component: [ANTIFRAUD::alert_component].\"\n                ANTIFRAUD::alert_component new_value\n                log local0. \"new Alert component: [ANTIFRAUD::alert_component].\"\n            }",
            return_value: "ANTIFRAUD::alert_component ; Returns error type according to alert_type.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_component (VALUE)?" },
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
