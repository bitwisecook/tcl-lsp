//! `LSN::inbound` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable inbound mapping for translation address and port associated with the cur",
            &["LSN::inbound disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
