//! `SIP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the URI of the request.",
            &["SIP::uri (URI_STRING)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
