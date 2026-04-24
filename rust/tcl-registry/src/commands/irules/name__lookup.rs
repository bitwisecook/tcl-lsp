//! `NAME::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NAME::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Performs DNS query for A or PTR record corresponding to a hostname o",
            &["NAME::lookup"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
