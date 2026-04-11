//! `ECA::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns NTLM authenticating username.",
            &["ECA::username"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
