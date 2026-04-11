//! `L7CHECK::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "L7CHECK::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or get L7 protocol value.",
            &["L7CHECK::protocol set VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
