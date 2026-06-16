//! `pwd` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pwd",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return current working directory",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        traits: Traits::RETURNS_PATH,
        ..CommandSpec::DEFAULT
    }
}
