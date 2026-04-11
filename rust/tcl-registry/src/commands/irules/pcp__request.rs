//! `PCP::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides access to the data sent in a PCP request.",
            &["PCP::request (opcode |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
