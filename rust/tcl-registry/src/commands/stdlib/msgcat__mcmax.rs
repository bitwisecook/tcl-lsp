//! `msgcat::mcmax` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcmax",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the maximum length of the translations of the given strings.",
            &["msgcat::mcmax ?src-string ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
