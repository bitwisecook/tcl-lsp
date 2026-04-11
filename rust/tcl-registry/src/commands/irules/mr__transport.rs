//! `MR::transport` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::transport",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name and type (virtual or config) of the transport used to configure",
            &["MR::transport"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
