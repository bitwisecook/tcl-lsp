//! `ASM::support_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::support_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the support id of the HTTP transaction.",
            &["ASM::support_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
