//! `PROFILE::diameter` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::diameter",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current value of the specified setting in an assigned DIAMETER profi",
            &["PROFILE::diameter ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
