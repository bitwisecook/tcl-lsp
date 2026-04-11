//! `UDP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allow client-side ingress to flow following a call to UDP::hold.",
            &["UDP::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
