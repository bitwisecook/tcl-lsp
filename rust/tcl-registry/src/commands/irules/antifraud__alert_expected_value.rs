//! `ANTIFRAUD::alert_expected_value` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_expected_value",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets expected (verified) value, for example in strong integrity check.",
            synopsis: &["ANTIFRAUD::alert_expected_value (VALUE)?"],
            snippet: "ANTIFRAUD::alert_expected_value ;\n                Returns expected (verified) value, for example in strong integrity check.\n\n            ANTIFRAUD::alert_expected_value VALUE ;\n                Sets expected (verified) value, for example in strong integrity check.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_expected_value.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert expected value: [ANTIFRAUD::alert_expected_value].\"\n                ANTIFRAUD::alert_expected_value new_value\n                log local0. \"new Alert expected value: [ANTIFRAUD::alert_expected_value].\"\n            }",
            return_value: "ANTIFRAUD::alert_expected_value ; Returns expected (verified) value, for example in strong integrity check.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_expected_value (VALUE)?" },
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
