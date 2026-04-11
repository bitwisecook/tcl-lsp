//! `TCP::bandwidth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::bandwidth",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the estimated bandwidth of the connection.",
            &["TCP::bandwidth"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
