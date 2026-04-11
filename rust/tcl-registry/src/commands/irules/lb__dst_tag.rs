//! `LB::dst_tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::dst_tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the destination tag for the current request.",
            &["LB::dst_tag"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
