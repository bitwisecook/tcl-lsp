//! `LSN::pool` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::pool",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Explicitly set the LSN pool used for translation.",
            &["LSN::pool LSN_POOL"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
