//! `RESOLVER::summarize` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::summarize",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a summary of the response.",
            &["RESOLVER::summarize DNS_MESSAGE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
