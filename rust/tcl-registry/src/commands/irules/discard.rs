//! `discard` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "discard",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the current packet or connection to be dropped/discarded.",
            &["discard"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
