//! `when` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        traits: Traits::LANGUAGE_KEYWORD | Traits::IS_EVENT_HANDLER | Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 6),
        hover: Some(HoverSnippet::brief(
            "Declare an iRules event handler block.",
            &["when EVENT { body }"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
