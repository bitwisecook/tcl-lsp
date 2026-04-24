//! `STATS::setmin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::setmin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Ensures that the value of a Statistics profile setting is at the most value.",
            &["STATS::setmin PROFILE_NAME FIELD_NAME (VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
