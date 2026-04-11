//! `AVR::disable_cspm_injection` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::disable_cspm_injection",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables CSPM injection for the current connection.",
            &["AVR::disable_cspm_injection"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
