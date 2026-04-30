//! `XLAT::src_nat_valid_range` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_nat_valid_range",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return a list of valid source-translation endpoint ranges.",
            &["XLAT::src_nat_valid_range"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
