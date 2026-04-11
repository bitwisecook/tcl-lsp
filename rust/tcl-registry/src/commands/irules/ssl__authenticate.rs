//! `SSL::authenticate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the current setting for authentication frequency or for the maximum de",
            &["SSL::authenticate (once | always | (depth DEPTH))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
