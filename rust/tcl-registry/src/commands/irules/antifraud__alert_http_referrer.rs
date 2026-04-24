//! `ANTIFRAUD::alert_http_referrer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_http_referrer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets alert HTTP referrer.",
            &["ANTIFRAUD::alert_http_referrer (VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
