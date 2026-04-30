//! `AAA::acct_send` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::acct_send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to send user accouting information to IVS(internal virtual ",
            &["AAA::acct_send VIRTUAL_SERVER ((('user-name' USERNAME)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
