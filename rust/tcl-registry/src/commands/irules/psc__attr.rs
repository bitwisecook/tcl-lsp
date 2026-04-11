//! `PSC::attr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::attr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets, sets or removes the custom attributes.",
            &["PSC::attr ((NAME) | (NAME VALUE))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
