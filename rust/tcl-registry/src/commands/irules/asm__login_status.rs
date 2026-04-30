//! `ASM::login_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::login_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Request status of the login session tracked by one of the login pages defined in",
            &["ASM::login_status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
