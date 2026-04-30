//! `DNS::last_act` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::last_act",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the action to perform if no DNS service handles this packet.",
            &["DNS::last_act ('allow' | 'drop' | 'reject' | 'hint' | 'noerror')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
