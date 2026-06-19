//! `image` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "create",
        arity: Arity::at_least(1),
        detail: "Create a new image of the given type (photo or bitmap).",
        synopsis: "image create type ?name? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "Delete one or more images by name.",
        synopsis: "image delete ?name name ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "height",
        arity: Arity::exact(1),
        detail: "Return the height of the image in pixels.",
        synopsis: "image height name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inuse",
        arity: Arity::exact(1),
        detail: "Return whether the image is in use by any widgets.",
        synopsis: "image inuse name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return a list of the names of all existing images.",
        synopsis: "image names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the type of the image (e.g. photo or bitmap).",
        synopsis: "image type name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "types",
        arity: Arity::exact(0),
        detail: "Return a list of the valid image types.",
        synopsis: "image types",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "width",
        arity: Arity::exact(1),
        detail: "Return the width of the image in pixels.",
        synopsis: "image width name",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "image option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "image",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate images.",
            synopsis: &[
                "image create type ?name? ?option value ...?",
                "image delete ?name name ...?",
                "image height name",
                "image inuse name",
                "image names",
                "image type name",
                "image types",
                "image width name",
            ],
            snippet: "",
            source: "Tk man page image.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
