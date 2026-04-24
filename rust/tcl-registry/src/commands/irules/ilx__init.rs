//! `ILX::init` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::init",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a handle to a running ILX plugin extension.",
            &["ILX::init (EXTENSION | (PLUGIN EXTENSION))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
