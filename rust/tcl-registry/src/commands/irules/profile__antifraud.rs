//! `PROFILE::antifraud` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::antifraud",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a ANTIFRAUD profile setting.",
            &["PROFILE::antifraud ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
