//! `LB::queue` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns queue information.",
            &["LB::queue queued"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
