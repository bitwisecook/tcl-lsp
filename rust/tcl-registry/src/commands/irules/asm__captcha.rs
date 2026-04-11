//! `ASM::captcha` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::captcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Responds to the client with a CAPTCHA challenge.",
            &["ASM::captcha"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
