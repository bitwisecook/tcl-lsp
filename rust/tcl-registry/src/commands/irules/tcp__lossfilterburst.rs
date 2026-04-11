//! `TCP::lossfilterburst` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterburst",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the TCP Loss Ignore Burst Parameter.",
            &["TCP::lossfilterburst"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
