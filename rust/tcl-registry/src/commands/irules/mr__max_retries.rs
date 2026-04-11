//! `MR::max_retries` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::max_retries",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the number of retries allows for this router instance.",
            &["MR::max_retries"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
