//! `WS::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to collect payload of current Websocket frame.",
            &["WS::collect ('frame' (LENGTH)? )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
