//! `IP::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::protocol",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the IP protocol value.",
            &["IP::protocol"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
