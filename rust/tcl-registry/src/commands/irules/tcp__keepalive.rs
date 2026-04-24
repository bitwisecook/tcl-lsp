//! `TCP::keepalive` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::keepalive",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set/Get the TCP Keep-Alive Interval.",
            &["TCP::keepalive (KEEP_ALIVE_INTERVAL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
