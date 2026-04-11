//! `ACCESS::restrict_irule_events` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::restrict_irule_events",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable or disable HTTP and higher layer iRule events for the internal APM access",
            &["ACCESS::restrict_irule_events (enable | disable)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
