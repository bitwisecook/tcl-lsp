//! `HTTP2::push` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::push",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Accepts a resource as a parameter that can be pushed to the client using PUSH_PR",
            &["HTTP2::push <uri> ?-priority num? ?-nohost? <request headers>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
