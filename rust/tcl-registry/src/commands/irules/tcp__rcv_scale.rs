//! `TCP::rcv_scale` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_scale",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the receive window scale advertised by the remote host.",
            &["TCP::rcv_scale"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
