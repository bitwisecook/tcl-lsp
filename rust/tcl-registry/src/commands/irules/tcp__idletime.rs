//! `TCP::idletime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::idletime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the TCP Idle Timeout.",
            &["TCP::idletime IDLE_TIME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
