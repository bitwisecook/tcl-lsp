//! `check` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the iRule validation level.",
            &["check"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
