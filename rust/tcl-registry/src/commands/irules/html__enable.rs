//! `HTML::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable the processing of HTML for this transaction.",
            &["HTML::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
