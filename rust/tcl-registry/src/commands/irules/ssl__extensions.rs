//! `SSL::extensions` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::extensions",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or manipulates SSL extensions.",
            &["SSL::extensions (count |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
