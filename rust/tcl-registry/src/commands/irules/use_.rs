//! `use` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "use",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "A BIG-IP 4.X statement.",
            &["use clone pool POOL_OBJ (member IP_ADDR)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
