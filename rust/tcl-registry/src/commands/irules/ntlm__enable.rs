//! `NTLM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NTLM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables processing for NTLM.",
            &["NTLM::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
