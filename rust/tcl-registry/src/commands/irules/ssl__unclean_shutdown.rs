//! `SSL::unclean_shutdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::unclean_shutdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the value of the Unclean Shutdown setting.",
            &["SSL::unclean_shutdown (enable | disable)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
