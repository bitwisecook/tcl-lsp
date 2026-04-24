//! `BOTDEFENSE::captcha_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::captcha_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the status of the user's answer to the CAPTCHA challenge.",
            &["BOTDEFENSE::captcha_status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
