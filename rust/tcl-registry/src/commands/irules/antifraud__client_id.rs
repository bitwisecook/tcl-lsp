//! `ANTIFRAUD::client_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::client_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns client id collected on client side.",
            synopsis: &["ANTIFRAUD::client_id"],
            snippet: "Returns client id collected on client side.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__client_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"client id: [ANTIFRAUD::client_id].\"\n            }",
            return_value: "Returns client id collected on client side.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::client_id" },
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
