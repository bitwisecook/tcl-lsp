//! `PSC::aaa_reporting_interval` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::aaa_reporting_interval",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set AAA reporting interval.",
            &["PSC::aaa_reporting_interval (INTERVAL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
