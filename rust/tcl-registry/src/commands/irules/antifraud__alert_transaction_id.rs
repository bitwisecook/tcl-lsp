//! `ANTIFRAUD::alert_transaction_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_transaction_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets alert HTTP transaction ID.",
            synopsis: &["ANTIFRAUD::alert_transaction_id (VALUE)?"],
            snippet: "ANTIFRAUD::alert_transaction_id ;\n                Returns alert HTTP transaction ID.\n\n            ANTIFRAUD::alert_transaction_id VALUE ;\n                Sets alert HTTP transaction ID.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert transaction ID: [ANTIFRAUD::alert_transaction_id].\"\n                ANTIFRAUD::alert_transaction_id new_value\n                log local0. \"new Alert transaction ID: [ANTIFRAUD::alert_transaction_id].\"\n            }",
            return_value: "ANTIFRAUD::alert_transaction_id ; Returns alert HTTP transaction ID.",
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
            synopsis: "ANTIFRAUD::alert_transaction_id (VALUE)?",
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
