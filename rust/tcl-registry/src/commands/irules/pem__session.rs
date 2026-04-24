//! `PEM::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to create, delete or retreive information of a PEM sessi",
            &["PEM::session config policy ((get IP_ADDR) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
