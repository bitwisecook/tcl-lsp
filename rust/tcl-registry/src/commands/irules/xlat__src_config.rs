//! `XLAT::src_config` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_config",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieve the source-translation configuration.",
            &["XLAT::src_config"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
