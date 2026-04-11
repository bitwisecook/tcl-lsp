//! `BOTDEFENSE::client_class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::client_class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the classification of the client based on the current request and its br",
            &["BOTDEFENSE::client_class"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
