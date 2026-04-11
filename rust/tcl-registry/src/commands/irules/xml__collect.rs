//! `XML::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::collect"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
