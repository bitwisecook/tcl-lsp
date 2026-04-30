//! `TAP::config` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::config",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current value of the specified setting in an assigned TAP Applicatio",
            &["TAP::config APPLICATION ENTITY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
