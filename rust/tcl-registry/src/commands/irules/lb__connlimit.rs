//! `LB::connlimit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::connlimit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the connection limit for virtual/node/poolmember.",
            &["LB::connlimit ('virtual' | 'node' | 'poolmember') ?limit <value>? ?key <value>?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
