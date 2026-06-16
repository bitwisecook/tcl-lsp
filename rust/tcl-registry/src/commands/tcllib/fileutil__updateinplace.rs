//! `fileutil::updateInPlace` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::updateInPlace ?options? fileName cmdOrBody",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::updateInPlace",
        dialects: None,
        arity: Arity::at_least(2),
        // SYNC3: the trailing `cmdOrBody` argument is invoked as a
        // command prefix with the file contents appended at runtime.
        // Static arity checks must relax the proc's required arity
        // by 1 when checking the callback (see `e30b6ae9`, `#308`).
        body_arg_implicit_args: 1,
        hover: Some(HoverSnippet {
            summary: "Update a file in place using a command.",
            synopsis: &["fileutil::updateInPlace ?options? fileName cmdOrBody"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
