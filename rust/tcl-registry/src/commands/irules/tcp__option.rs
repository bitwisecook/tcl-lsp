//! `TCP::option` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Retrieves or changes TCP header options.",
            &["TCP::option get <kind>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
