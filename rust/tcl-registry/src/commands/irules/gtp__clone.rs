//! `GTP::clone` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::clone",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a cloned copy of the GTP message.",
            &["GTP::clone (MESSAGE_VAR)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
