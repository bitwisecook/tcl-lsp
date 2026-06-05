//! `COMPRESS::nodelay` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::nodelay",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `COMPRESS::nodelay`.",
            synopsis: &["COMPRESS::nodelay (request | response)?"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__nodelay.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "COMPRESS::nodelay (request | response)?",
        }],
        ..CommandSpec::DEFAULT
    }
}
