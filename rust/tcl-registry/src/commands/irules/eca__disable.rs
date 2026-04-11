//! `ECA::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the plugin in the flow.",
            &["ECA::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
