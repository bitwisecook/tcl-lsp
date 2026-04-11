//! `XLAT::src_endpoint_reservation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_endpoint_reservation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "XLAT:src_endpoint_reservation",
            &["XLAT::src_endpoint_reservation create"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
