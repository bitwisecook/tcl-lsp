//! `MR::instance` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::instance",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name of the current mr_router instance.",
            &["MR::instance"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
