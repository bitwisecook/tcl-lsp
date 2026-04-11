//! `HTTP::close` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::close",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Closes the HTTP connection.",
            &["HTTP::close"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
