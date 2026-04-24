//! `WS::masking` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::masking",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command determines the behavior of Websocket processing.",
            &["WS::masking ( 'preserve' | 'remask' )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
