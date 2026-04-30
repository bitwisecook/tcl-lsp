//! `ACCESS::flowid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::flowid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the flow id for SSL Orchestrator using APM logging framework.",
            &["ACCESS::flowid (FID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
