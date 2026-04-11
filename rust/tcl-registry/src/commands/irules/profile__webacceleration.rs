//! `PROFILE::webacceleration` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::webacceleration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an web acceleration profile setting.",
            &["PROFILE::webacceleration ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
