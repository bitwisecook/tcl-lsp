//! `WS::frame` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::frame",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to perform various operations on a Websocket frame, dete",
            &["WS::frame <subcommand> ?args?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
