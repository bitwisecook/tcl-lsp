//! `BIGTCP::release_flow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BIGTCP::release_flow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases a flow from BIGTCP's control.",
            &["BIGTCP::release_flow"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
