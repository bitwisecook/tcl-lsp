//! `listen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "listen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets up a related ephemeral listener to allow an incoming related connection to ",
            &["listen (<'proto' UNSIGNED_SHORT> |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
