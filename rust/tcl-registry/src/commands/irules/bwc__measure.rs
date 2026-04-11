//! `BWC::measure` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::measure",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to measure rate for a particular traffic flow or flows b",
            &["BWC::measure ( ('start' | 'stop') |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
