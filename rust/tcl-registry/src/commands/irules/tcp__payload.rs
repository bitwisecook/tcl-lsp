//! `TCP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 4),
        hover: Some(HoverSnippet::brief(
            "Returns or changes the data collected by TCP::collect.",
            &["TCP::payload ?<size>?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
