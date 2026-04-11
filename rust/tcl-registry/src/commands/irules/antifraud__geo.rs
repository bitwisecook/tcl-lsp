//! `ANTIFRAUD::geo` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::geo",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns L3 geoIP and geolocation collected by client.",
            &["ANTIFRAUD::geo"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
