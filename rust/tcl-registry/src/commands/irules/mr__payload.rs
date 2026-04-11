//! `MR::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Access data collected using MR::collect command.",
            &["MR::payload ( 'length' )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
