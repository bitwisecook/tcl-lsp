//! `ASM::violation_data` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::violation_data",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command exposes violation data using a multiple buffers instance.",
            &["ASM::violation_data"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
