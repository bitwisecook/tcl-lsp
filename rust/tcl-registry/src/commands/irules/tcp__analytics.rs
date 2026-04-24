//! `TCP::analytics` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::analytics",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to ca",
            &["TCP::analytics (enable | disable | key (KEY)?)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
