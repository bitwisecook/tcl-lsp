//! `LINE::set` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINE::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `LINE::set`.",
            synopsis: &["LINE::set"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/LINE__set.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LINE::set",
        }],
        ..CommandSpec::DEFAULT
    }
}
