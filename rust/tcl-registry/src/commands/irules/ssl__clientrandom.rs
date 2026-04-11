//! `SSL::clientrandom` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::clientrandom",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the ClientRandom value from the Client hello.",
            &["SSL::clientrandom"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
