//! `ECA::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables the plugin in the flow.",
            &["ECA::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
