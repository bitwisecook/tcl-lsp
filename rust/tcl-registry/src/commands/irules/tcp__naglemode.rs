//! `TCP::naglemode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::naglemode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns setting of Nagle mode.",
            &["TCP::naglemode"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
