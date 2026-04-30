//! `CACHE::useragent` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::useragent",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the useragent value used by the cache to reference the cached content.",
            &["CACHE::useragent AGENT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
