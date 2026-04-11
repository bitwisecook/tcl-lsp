//! `IKE::san_ipadd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::san_ipadd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "something",
            &["IKE::san_ipadd (ANY_CHARS)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
