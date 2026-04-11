//! `PROFILE::xml` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::xml",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an XML profile setting.",
            &["PROFILE::xml ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
