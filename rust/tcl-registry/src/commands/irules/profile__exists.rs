//! `PROFILE::exists` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::exists",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Determine if a profile is configured on a virtual server.",
            &["PROFILE::exists TYPE (NAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
