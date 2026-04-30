//! `OFFBOX::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "OFFBOX::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Performs a request with a payload to certain service.", &["OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEO"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
