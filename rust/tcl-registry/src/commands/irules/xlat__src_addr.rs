//! `XLAT::src_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieve the source translation address.",
            &["XLAT::src_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
