//! `AAA::auth_send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to send user authentication information to IVS(internal vir",
            &["AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
