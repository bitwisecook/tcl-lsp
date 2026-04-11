//! `SSL::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Drops a session from the SSL session cache.",
            &["SSL::session invalidate ( drop | nodrop )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
