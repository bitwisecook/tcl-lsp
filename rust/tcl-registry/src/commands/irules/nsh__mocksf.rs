//! `NSH::mocksf` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::mocksf",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set option to mock SF functionality for NSH.",
            &["NSH::mocksf"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
