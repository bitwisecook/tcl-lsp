//! `DOSL7::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables blocking and detection of DoS attacks according to the ASM security pol",
            &["DOSL7::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
