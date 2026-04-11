//! `BWC::pps` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::pps",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to modify pps value for the policy.",
            &["BWC::pps BW_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
