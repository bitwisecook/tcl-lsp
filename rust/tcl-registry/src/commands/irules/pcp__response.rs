//! `PCP::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides access to the data in a PCP response packet.",
            &["PCP::response (opcode |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
