//! `fileutil::relativeUrl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::relativeUrl",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Compute a relative URL path.",
            &["fileutil::relativeUrl base dst"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
