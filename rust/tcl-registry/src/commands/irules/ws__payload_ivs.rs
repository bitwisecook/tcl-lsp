//! `WS::payload_ivs` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload_ivs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies name of the Internal Virtual Server (IVS) that will process websocket ",
            &["WS::payload_ivs IVSNAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
