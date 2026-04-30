//! `XML::soap` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::soap",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::soap"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
