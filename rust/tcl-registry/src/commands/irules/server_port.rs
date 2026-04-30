//! `server_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "server_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TCP port/service number of the specified server.",
            &["server_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
