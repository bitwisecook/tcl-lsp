//! `fileutil::updateInPlace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::updateInPlace",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        // SYNC3: the trailing `cmdOrBody` argument is invoked as a
        // command prefix with the file contents appended at runtime.
        // Static arity checks must relax the proc's required arity
        // by 1 when checking the callback (see `e30b6ae9`, `#308`).
        body_arg_implicit_args: 1,
        hover: Some(HoverSnippet::brief(
            "Update a file in place using a command.",
            &["fileutil::updateInPlace ?options? fileName cmdOrBody"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
