//! `bl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bl",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("List all breakpoints.", &["bl"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
