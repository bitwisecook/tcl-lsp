//! `MR::equivalent_transport` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::equivalent_transport",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the transport that is usable as an equivalent transport.",
            &["MR::equivalent_transport"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
