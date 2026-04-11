//! `XML::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
