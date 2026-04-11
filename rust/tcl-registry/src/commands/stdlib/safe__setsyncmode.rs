//! `safe::setSyncMode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::setSyncMode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Set or query the synchronous-source mode for a safe interpreter.",
            &["safe::setSyncMode ?child? ?boolean?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
