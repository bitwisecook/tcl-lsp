//! `SSL::profile` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Switch between different SSL profiles.",
            &["SSL::profile PROFILE_OBJ"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
