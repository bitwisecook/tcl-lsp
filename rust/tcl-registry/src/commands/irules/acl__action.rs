//! `ACL::action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACL::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or retrieves the current ACL action.",
            &["ACL::action (default |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
