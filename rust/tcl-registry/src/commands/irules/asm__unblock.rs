//! `ASM::unblock` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::unblock",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the blocking action for a request that had blocking violation.",
            &["ASM::unblock"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
