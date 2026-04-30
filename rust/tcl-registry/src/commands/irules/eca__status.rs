//! `ECA::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns NTLM authentication result.",
            &["ECA::status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
