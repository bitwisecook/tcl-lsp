//! `HTTP::retry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::retry",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Resends a request to a server.",
            &["HTTP::retry ('-reset')? HTTP_REQUEST"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
