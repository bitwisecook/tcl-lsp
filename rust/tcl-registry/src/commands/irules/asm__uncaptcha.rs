//! `ASM::uncaptcha` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::uncaptcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Overrides the CAPTCHA action.",
            &["ASM::uncaptcha"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
