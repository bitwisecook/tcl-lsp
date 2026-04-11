//! `MR::peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::peer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Defines a peer to use for routing a message to.",
            &["MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
