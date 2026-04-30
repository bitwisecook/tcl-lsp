//! `create_net` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_net",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Create a new net.",
            &["create_net net_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
