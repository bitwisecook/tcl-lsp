//! `PLUGIN::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PLUGIN::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: removed",
            &["PLUGIN::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
