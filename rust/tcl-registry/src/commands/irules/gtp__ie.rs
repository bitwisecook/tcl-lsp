//! `GTP::ie` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::ie",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This set of commands allows for the parsing and interpretation of GTP IE element",
            &["GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
