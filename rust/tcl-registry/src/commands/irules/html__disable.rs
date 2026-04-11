//! `HTML::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable the processing of HTML for this transaction.",
            &["HTML::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
