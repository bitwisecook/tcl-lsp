//! `HA::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HA::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns true or false based on whether the unit the command is executed on is ac",
            &["HA::status (active | standby)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
