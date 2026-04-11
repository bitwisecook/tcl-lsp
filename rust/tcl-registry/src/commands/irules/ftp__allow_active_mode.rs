//! `FTP::allow_active_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::allow_active_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the state of allow active mode.",
            &["FTP::allow_active_mode (enable | disable)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
