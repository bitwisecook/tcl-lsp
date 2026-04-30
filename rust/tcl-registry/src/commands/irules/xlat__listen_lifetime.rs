//! `XLAT::listen_lifetime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen_lifetime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set/Get the listener lifetime.",
            &["XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
