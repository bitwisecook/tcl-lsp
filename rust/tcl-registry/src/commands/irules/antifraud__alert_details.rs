//! `ANTIFRAUD::alert_details` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_details",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets alert details.",
            synopsis: &["ANTIFRAUD::alert_details (VALUE)?"],
            snippet: "ANTIFRAUD::alert_details ;\n                Returns alert details.\n\n            ANTIFRAUD::alert_details VALUE ;\n                Sets alert details.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_details.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert details: [ANTIFRAUD::alert_details].\"\n                ANTIFRAUD::alert_details new_value\n                log local0. \"new Alert details: [ANTIFRAUD::alert_details].\"\n            }",
            return_value: "ANTIFRAUD::alert_details ; Returns alert details.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_details (VALUE)?" },
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
