//! `SSL::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collect plaintext data after SSL offloading.",
            &["SSL::collect (LENGTH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
