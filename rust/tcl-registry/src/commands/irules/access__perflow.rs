//! `ACCESS::perflow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::perflow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns perflow variable value",
            &["ACCESS::perflow get KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
