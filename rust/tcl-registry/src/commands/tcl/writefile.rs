//! `writeFile` — write contents to a text or binary file (Tcl 9.0+, TIP 670).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "writeFile filename ?text|binary? contents",
}];

/// Command spec for `writeFile`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "writeFile",
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(2, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Write contents to a file.",
            synopsis: &["writeFile filename ?text|binary? contents"],
            snippet: "",
            source: "Tcl man page library.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
