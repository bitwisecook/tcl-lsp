//! `TCP::proxybufferlow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybufferlow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets proxy buffer low threshold.",
            &["TCP::proxybufferlow"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
