//! `CACHE::accept_encoding` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::accept_encoding",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the accept_encoding value used by the cache to store the cached conten",
            &["CACHE::accept_encoding ENCODING_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
