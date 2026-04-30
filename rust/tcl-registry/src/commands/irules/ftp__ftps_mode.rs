//! `FTP::ftps_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::ftps_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the activation mode for FTPS.",
            &["FTP::ftps_mode (disallow | allow | require)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
