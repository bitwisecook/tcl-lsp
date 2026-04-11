//! `ASM::deception` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::deception",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Mark a request as deceptive for further enforcement by asm",
            &["ASM::deception"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
