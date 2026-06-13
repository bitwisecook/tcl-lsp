//! `ANTIFRAUD::alert_transaction_data` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_transaction_data",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets key-value list of all parameters marked to be attached.",
            synopsis: &["ANTIFRAUD::alert_transaction_data (VALUE)?"],
            snippet: "ANTIFRAUD::alert_transaction_data ;\n                Returns key-value list of all parameters marked to be attached.\n\n            ANTIFRAUD::alert_transaction_data VALUE ;\n                Sets key-value list of all parameters marked to be attached.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_data.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert transaction data: [ANTIFRAUD::alert_transaction_data].\"\n                ANTIFRAUD::alert_transaction_data new_value\n                log local0. \"new Alert transaction data: [ANTIFRAUD::alert_transaction_data].\"\n            }",
            return_value: "ANTIFRAUD::alert_transaction_data ; Returns key-value list of all parameters marked to be attached.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_transaction_data (VALUE)?" },
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
