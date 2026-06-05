//! `AM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `AM::disable`.",
            synopsis: &["AM::disable"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/AM__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AM::disable",
        }],
        ..CommandSpec::DEFAULT
    }
}
