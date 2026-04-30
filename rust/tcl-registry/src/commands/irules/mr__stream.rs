//! `MR::stream` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::stream",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Start egressing bytes previously collected and stored.",
            &["MR::stream ( 'end' )? (BYTES)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
