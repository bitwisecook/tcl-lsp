//! `CACHE::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the URI value used by the cache to store the cached content.",
            &["CACHE::uri URI_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
