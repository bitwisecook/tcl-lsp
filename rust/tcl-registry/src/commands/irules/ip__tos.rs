//! `IP::tos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::tos",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns (or sets) the ToS value encoded within a packet.",
            &["IP::tos (clientside | serverside)? (IP_TOS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
