//! `local_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "local_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: use TCP::local_port instead",
            &["local_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
