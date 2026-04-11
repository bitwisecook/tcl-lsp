//! `XLAT::src_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieve the source translation port.",
            &["XLAT::src_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
