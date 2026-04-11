//! `PSC::user_name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::user_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set user name.",
            &["PSC::user_name (USERNAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
