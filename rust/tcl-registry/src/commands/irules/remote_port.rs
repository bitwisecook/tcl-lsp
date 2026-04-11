//! `remote_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remote_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: use TCP::remote_port instead",
            &["remote_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
