//! `IPFIX::destination` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "IPFIX::destination Provides the ability to manage IPFIX logging destinations and",
            &["IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
