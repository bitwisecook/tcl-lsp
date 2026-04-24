//! `ASM::microservice` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::microservice",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "request matched microservice",
            &["ASM::microservice"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
