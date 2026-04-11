//! `persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "persist",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the connection persistence type.",
            &["persist none"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
