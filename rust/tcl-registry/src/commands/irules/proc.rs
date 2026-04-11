//! `proc` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Define an iRule proc.",
            &["proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
