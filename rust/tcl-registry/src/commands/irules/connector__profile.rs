//! `CONNECTOR::profile` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get connector profile name.",
            &["CONNECTOR::profile"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
