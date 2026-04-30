//! `XML::eventid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::eventid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: XML profile deprecated",
            &["XML::eventid"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
