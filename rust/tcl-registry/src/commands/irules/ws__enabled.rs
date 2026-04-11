//! `WS::enabled` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::enabled",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to turn off WebSocket processing.",
            &["WS::enabled ( 'false' )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
