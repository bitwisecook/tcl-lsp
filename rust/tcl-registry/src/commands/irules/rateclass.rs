//! `rateclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rateclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Selects the specified rate class to use when transmitting packets.",
            &["rateclass RATE_CLASS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
