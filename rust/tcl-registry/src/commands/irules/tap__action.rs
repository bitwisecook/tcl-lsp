//! `TAP::action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Returns or updates security token action.", &["TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset |"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
