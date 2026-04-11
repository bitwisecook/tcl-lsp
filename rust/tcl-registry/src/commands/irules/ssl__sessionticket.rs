//! `SSL::sessionticket` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionticket",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the session ticket associated with the SSL flow.",
            &["SSL::sessionticket"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
