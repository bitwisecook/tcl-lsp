//! `MR::connect_back_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connect_back_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets connect_back_port for the current connection.",
            &["MR::connect_back_port (NONNEGATIVE_INTEGER)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
