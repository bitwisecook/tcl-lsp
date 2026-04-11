//! `MR::flow_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::flow_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a unique identifier for the current connection.",
            &["MR::flow_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
