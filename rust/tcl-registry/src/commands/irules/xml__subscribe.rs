//! `XML::subscribe` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::subscribe",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::subscribe"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
