//! `CLASSIFY::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Assigns username to the flow.",
            &["CLASSIFY::username USERNAME (CONTEXT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
