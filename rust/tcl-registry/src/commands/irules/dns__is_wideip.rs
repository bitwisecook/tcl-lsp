//! `DNS::is_wideip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::is_wideip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns status (true/false) if a string is a configured wide IP.",
            &["DNS::is_wideip DNS_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
