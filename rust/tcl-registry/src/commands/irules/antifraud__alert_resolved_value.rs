//! `ANTIFRAUD::alert_resolved_value` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_resolved_value",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets resolved (actual) value.",
            synopsis: &["ANTIFRAUD::alert_resolved_value (VALUE)?"],
            snippet: "ANTIFRAUD::alert_resolved_value ;\n                Returns resolved (actual) value.\n\n            ANTIFRAUD::alert_resolved_value VALUE ;\n                Sets resolved (actual) value.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_resolved_value.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert resolved value: [ANTIFRAUD::alert_resolved_value].\"\n                ANTIFRAUD::alert_resolved_value new_value\n                log local0. \"new Alert resolved value: [ANTIFRAUD::alert_resolved_value].\"\n            }",
            return_value: "ANTIFRAUD::alert_resolved_value ; Returns resolved (actual) value.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_resolved_value (VALUE)?" },
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
