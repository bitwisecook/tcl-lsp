//! `XML::parse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::parse"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
