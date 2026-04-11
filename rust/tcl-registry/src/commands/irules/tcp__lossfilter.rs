//! `TCP::lossfilter` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilter",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the TCP Loss Ignore Parameters.",
            &["TCP::lossfilter TCP_IGNORE_RATE TCP_IGNORE_BURST"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
