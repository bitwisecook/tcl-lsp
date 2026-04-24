//! `ASM::severity` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::severity",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the overall severity of the violations found in the transaction (both re",
            &["ASM::severity"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
