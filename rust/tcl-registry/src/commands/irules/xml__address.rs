//! `XML::address` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::address"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
