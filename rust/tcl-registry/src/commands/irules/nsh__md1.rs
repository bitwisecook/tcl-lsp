//! `NSH::md1` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::md1",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets/Get the MD1 context for NSH.",
            &["NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
