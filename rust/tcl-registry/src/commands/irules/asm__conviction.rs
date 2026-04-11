//! `ASM::conviction` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::conviction",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Inject conviction honey traps in case of behavioral enforcement is enabled",
            &["ASM::conviction"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
