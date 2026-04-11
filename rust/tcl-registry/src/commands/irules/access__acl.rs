//! `ACCESS::acl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::acl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Poll or enforce ACLs in your connections.",
            &["ACCESS::acl (result | matched | lookup | (eval ACL_NAME))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
