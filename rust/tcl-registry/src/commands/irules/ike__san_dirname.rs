//! `IKE::san_dirname` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::san_dirname",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "something",
            &["IKE::san_dirname (ANY_CHARS)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
