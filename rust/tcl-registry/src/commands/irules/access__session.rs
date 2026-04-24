//! `ACCESS::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Access or manipulate session information.", &["ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
