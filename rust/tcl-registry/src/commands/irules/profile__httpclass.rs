//! `PROFILE::httpclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::httpclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: use PROFILE::http instead",
            &["PROFILE::httpclass"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
