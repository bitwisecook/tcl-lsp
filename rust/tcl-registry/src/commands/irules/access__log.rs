//! `ACCESS::log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Logs a message using APM logging framework.",
            &["ACCESS::log (COMPONENT_LOGLEVEL)? MSG"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
