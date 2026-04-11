//! `ASM::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieves or replaces the payload collected by ASM.",
            &["ASM::payload (LENGTH | (OFFSET LENGTH))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
