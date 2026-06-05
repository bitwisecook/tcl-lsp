//! `DEMANGLE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DEMANGLE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `DEMANGLE::disable`.",
            synopsis: &["DEMANGLE::disable"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/DEMANGLE__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DEMANGLE::disable",
        }],
        ..CommandSpec::DEFAULT
    }
}
