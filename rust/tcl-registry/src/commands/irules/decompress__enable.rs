//! `DECOMPRESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DECOMPRESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable DECOMPRESS feature on current flow.",
            synopsis: &["DECOMPRESS::enable (request | response)?"],
            snippet: "Enable DECOMPRESS feature on current flow.",
            source: "https://clouddocs.f5.com/api/irules/DECOMPRESS__enable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DECOMPRESS::enable (request | response)?",
        }],
        ..CommandSpec::DEFAULT
    }
}
