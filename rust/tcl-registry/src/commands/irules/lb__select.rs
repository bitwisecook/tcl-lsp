//! `LB::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forces a load balancing selection and returns the result.",
            &["LB::select"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
