//! `MR::connection_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the connection mode.",
            &["MR::connection_mode"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
