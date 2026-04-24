//! `DIAMETER::skip_capabilities_exchange` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::skip_capabilities_exchange",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Instructs DIAMETER protocol to skip capabilities exchange when establishing a pe",
            &["DIAMETER::skip_capabilities_exchange ( HOSTNAME )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
