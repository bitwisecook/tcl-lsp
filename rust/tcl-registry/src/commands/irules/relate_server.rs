//! `relate_server` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "relate_server",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets up a related established connection.",
            &["relate_server CONFIG"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
