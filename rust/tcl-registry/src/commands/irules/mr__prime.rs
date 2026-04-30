//! `MR::prime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::prime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("establishes an outgoing connection to the specified host or hosts using the spec", &["MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
