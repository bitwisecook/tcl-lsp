//! `ASM::violation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::violation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the list of violations found in the request or response together with de",
            &["ASM::violation (count | names | attack_types | details | rating)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
