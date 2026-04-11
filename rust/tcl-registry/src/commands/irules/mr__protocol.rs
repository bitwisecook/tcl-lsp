//! `MR::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns generic, sip or diameter.",
            &["MR::protocol"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
