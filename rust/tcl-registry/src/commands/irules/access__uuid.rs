//! `ACCESS::uuid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::uuid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enumerates the session IDs that belongs to a specified uuid key by the order of ",
            &["ACCESS::uuid getsid SESSION_ID"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
