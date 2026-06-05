//! `CONNECTOR::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable all the connectors on chain.",
            synopsis: &["CONNECTOR::enable"],
            snippet: "Enable all the connectors on chain.",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__enable.html",
            examples: "when CLIENTSSL_DATA {\n                CONNECTOR::enable\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CONNECTOR::enable",
        }],
        ..CommandSpec::DEFAULT
    }
}
