//! `reject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "reject",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Causes the connection to be rejected.",
            &["reject"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
