//! `MR::return` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::return",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current message to the originating connection.",
            &["MR::return"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
