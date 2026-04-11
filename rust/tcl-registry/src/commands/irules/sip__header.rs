//! `SIP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets SIP header information.",
            &["SIP::header SIP_HEADER_NAME (INDEX)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
