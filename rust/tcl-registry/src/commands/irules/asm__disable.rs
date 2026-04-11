//! `ASM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables plugin processing on the connection.",
            &["ASM::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
