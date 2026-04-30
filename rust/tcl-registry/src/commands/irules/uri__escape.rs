//! `URI::escape` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::escape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Percent-encodes a URI string (alias for URI::encode).",
            &["URI::escape URI_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
