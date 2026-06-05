//! `ANTIFRAUD::disable_auto_transactions` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_auto_transactions",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables automatic transactions for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_auto_transactions"],
            snippet: "Disables automatic transactions for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_auto_transactions.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Disable-AutoTransactions\" ] } {\n                    ANTIFRAUD::disable_auto_transactions\n                    log local0. \"Automatic Transactions disabled\"\n                }\n            }",
            return_value: "Disables automatic transactions for the current transaction.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::disable_auto_transactions" },
        ],
        ..CommandSpec::DEFAULT
    }
}
