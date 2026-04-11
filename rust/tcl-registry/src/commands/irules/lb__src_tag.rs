//! `LB::src_tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::src_tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the source tag for the current request.",
            &["LB::src_tag"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
