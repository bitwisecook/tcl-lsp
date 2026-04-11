//! `node` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "node",
        traits: Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Route traffic directly to a specific node.",
            &["node ip_addr ?service_port?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
