//! `ANTIFRAUD::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables the anti-fraud plugin.",
            synopsis: &["ANTIFRAUD::disable"],
            snippet: "Disables the anti-fraud plugin.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable.html",
            examples: "when HTTP_REQUEST {\n                # Disable request with Antifraud-Disable header (bypass antifraud plugin)\n                if { [HTTP::header exists \"Antifraud-Disable\" ] } {\n                    ANTIFRAUD::disable\n                }\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::disable" },
        ],
        ..CommandSpec::DEFAULT
    }
}
