//! `TCP::lossfilterrate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterrate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the TCP Loss Ignore Rate Parameter.",
            &["TCP::lossfilterrate"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
