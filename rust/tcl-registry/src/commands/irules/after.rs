//! `after` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Execute iRules code after a set period of delay.",
            &["after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
