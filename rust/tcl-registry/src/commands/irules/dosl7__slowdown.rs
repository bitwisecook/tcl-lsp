//! `DOSL7::slowdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::slowdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Adds source IP extracted from current connection to greylist.",
            &["DOSL7::slowdown RATE TIMEOUT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
