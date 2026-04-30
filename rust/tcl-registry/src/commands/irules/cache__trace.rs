//! `CACHE::trace` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::trace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Dump the list of cached objects for a HTTP profile where RAM Cache is enabled.",
            &["CACHE::trace (MAX)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
