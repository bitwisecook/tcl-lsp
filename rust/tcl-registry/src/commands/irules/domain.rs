//! `domain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Parses the specified string as a dotted domain name and returns the last portion",
            &["domain DOMAIN COUNT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
