//! `AM::expires` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::expires",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::expires`.",
            &["AM::expires"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
