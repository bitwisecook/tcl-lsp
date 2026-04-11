//! `DIAMETER::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the session-id attribute-value pair.",
            &["DIAMETER::session (SESSION_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
