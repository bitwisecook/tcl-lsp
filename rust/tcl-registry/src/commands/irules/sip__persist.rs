//! `SIP::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or replaces the persistence key being used for the current message.",
            &["SIP::persist"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
