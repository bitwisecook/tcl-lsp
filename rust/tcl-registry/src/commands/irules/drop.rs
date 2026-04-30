//! `drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "drop",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the current packet or connection to be dropped/discarded.",
            &["drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
