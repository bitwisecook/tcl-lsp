//! `MESSAGE::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Various operations for a message's fields.",
            &["MESSAGE::field ( ('names') |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
