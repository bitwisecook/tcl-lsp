//! `PROFILE::list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns all the names of the profiles of the class asked for that are attached t",
            &["PROFILE::list 'auth'"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
