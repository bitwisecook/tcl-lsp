//! `ASM::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the name of the ASM security policy that was applied for the request.",
            &["ASM::policy"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
