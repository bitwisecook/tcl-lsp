//! `TCP::rcv_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the maximum allowed advertised window size by BIG-IP.",
            &["TCP::rcv_size"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
