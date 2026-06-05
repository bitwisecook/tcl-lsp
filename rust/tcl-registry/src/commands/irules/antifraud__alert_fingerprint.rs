//! `ANTIFRAUD::alert_fingerprint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_fingerprint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets fingerprint data collected on client side.",
            synopsis: &["ANTIFRAUD::alert_fingerprint (VALUE)?"],
            snippet: "ANTIFRAUD::alert_fingerprint ;\n                Returns fingerprint data collected on client side.\n\n            ANTIFRAUD::alert_fingerprint VALUE ;\n                Sets fingerprint data collected on client side.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_fingerprint.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert fingerprint: [ANTIFRAUD::alert_fingerprint].\"\n                ANTIFRAUD::alert_fingerprint new_value\n                log local0. \"new Alert fingerprint: [ANTIFRAUD::alert_fingerprint].\"\n            }",
            return_value: "ANTIFRAUD::alert_fingerprint ; Returns fingerprint data collected on client side.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_fingerprint (VALUE)?" },
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
