//! `XML::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the XML plugin from passthrough to full patching mode.",
            &["XML::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
