//! `ACCESS2::access2_proc` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS2::access2_proc",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to get the TCL procedure registered for currently executing",
            &["ACCESS2::access2_proc"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
