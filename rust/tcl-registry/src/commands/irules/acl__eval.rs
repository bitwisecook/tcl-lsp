//! `ACL::eval` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACL::eval",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enforce ACLs in your connections.",
            &["ACL::eval ('-l7')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
