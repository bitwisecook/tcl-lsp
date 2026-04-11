//! `GTP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows for the parsing of GTP header information.",
            &["GTP::header ('version' | 'type') ('-message' MESSAGE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
