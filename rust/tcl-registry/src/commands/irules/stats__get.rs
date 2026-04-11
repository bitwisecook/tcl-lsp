//! `STATS::get` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieves a setting value from a Statistics profile.",
            &["STATS::get PROFILE_NAME FIELD_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
