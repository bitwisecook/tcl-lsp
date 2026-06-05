//! `AM::application` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::application",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `AM::application`.",
            synopsis: &["AM::application"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/AM__application.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AM::application",
        }],
        ..CommandSpec::DEFAULT
    }
}
