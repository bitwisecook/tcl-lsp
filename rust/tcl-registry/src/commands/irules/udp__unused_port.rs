//! `UDP::unused_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::unused_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an unused UDP port for the specified IP tuple.",
            &["UDP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
