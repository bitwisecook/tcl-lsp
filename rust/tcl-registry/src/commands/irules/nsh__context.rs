//! `NSH::context` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::context",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets/Get the Context header for NSH.",
            &["NSH::context NSH_CONTEXT_IDX DIRECTION (CONTEXT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
