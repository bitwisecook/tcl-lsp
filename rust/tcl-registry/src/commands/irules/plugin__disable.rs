//! `PLUGIN::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PLUGIN::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: removed",
            &["PLUGIN::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
