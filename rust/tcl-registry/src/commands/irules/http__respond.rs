//! `HTTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send an immediate HTTP response from an iRule.",
            &["HTTP::respond <status> ?option value ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
