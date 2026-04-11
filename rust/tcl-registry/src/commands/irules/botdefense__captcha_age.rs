//! `BOTDEFENSE::captcha_age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::captcha_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the age of the CAPTCHA challenge in seconds.",
            &["BOTDEFENSE::captcha_age"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
