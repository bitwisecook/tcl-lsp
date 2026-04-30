//! `UDP::hold` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::hold",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Hold client ingress until UDP::release is called.",
            &["UDP::hold"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
