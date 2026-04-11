//! `LB::server` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::server",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Returns information about the currently selected server.", &["LB::server ?name | pool | route_domain | addr | port | priority | ratio | weight | ripeness?"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
