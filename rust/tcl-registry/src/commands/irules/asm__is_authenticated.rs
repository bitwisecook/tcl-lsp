//! `ASM::is_authenticated` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::is_authenticated",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Request login status of the user in the present request.",
            &["ASM::is_authenticated"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
