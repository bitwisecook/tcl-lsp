//! `LB::class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides the name of the traffic class that matched the connection",
            &["LB::class"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
