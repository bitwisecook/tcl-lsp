//! `MR::connection_instance` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_instance",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the connection instance and the number of connections.",
            &["MR::connection_instance"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
