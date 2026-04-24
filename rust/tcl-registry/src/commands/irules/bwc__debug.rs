//! `BWC::debug` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::debug",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used for troubleshooting a bwc policy instance.",
            &["BWC::debug ('start')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
