//! `MR::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases the data collected via MR::collect iRule command.",
            &["MR::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
