//! `ADAPT::service_down_action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::service_down_action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Sets or returns the service-down-action attribute.", &["ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
