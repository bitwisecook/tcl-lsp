//! `virtual` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "virtual",
        traits: Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name of the associated virtual server or selects another virtual ser",
            &["virtual"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
