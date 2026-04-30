//! `CACHE::userkey` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::userkey",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows users to add user-defined values to the key used by the cache to referenc",
            &["CACHE::userkey KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
