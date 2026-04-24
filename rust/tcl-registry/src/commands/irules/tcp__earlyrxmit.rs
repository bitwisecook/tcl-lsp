//! `TCP::earlyrxmit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::earlyrxmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Toggles TCP early retransmit.",
            &["TCP::earlyrxmit (BOOL_VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
