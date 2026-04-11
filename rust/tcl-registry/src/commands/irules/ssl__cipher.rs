//! `SSL::cipher` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cipher",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns SSL cipher information.",
            &["SSL::cipher bits"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
