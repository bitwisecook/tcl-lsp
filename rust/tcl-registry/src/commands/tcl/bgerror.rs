//! `bgerror` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bgerror",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Handle background errors",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
