//! `MR::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collect the specified amount of MR message payload data.",
            &["MR::collect (COLLECT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
