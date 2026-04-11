//! `DIAMETER::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the persistence key being used for the current message.",
            &["DIAMETER::persist"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
