//! `llookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "llookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Returns a list of values corresponding to the given key in a multimap.",
            &["llookup MMAP KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
