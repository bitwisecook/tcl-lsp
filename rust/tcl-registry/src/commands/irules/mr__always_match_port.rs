//! `MR::always_match_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::always_match_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the always_match_port mode for the router.",
            &["MR::always_match_port (BOOLEAN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
