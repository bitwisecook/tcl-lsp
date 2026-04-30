//! `mime::finalize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::finalize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Destroy a MIME part and free resources.",
            &["mime::finalize token ?-subordinates all?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
