//! `BWC::priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to create a priority class for a bwc policy.",
            &["BWC::priority (PRIORITY_CLASS WEIGHT)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
