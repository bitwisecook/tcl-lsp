//! `REST::send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REST::send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Send a rest request.",
            synopsis: &["REST::send"],
            snippet: "Send a rest request locally to the Big-IP REST Framework",
            source: "https://clouddocs.f5.com/api/irules/REST__send.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "REST::send -method METHOD URI ?BODY?",
        }],
        options: &[OptionSpec {
            name: "-method",
            takes_value: true,
            value_hint: "METHOD",
            detail: "HTTP method (GET, POST, PUT, DELETE, etc.).",
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
