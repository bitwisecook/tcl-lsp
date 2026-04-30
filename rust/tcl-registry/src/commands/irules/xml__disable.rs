//! `XML::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the XML plugin from full patching mode to passthrough.",
            &["XML::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
