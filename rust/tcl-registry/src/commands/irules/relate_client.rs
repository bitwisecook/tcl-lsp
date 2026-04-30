//! `relate_client` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "relate_client",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets up a related established connection.",
            &["relate_client CONFIG"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
