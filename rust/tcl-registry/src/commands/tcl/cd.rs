//! `cd` — change working directory.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cd ?dirName?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cd",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
hover: Some(HoverSnippet {
    summary: "Change working directory",
    synopsis: &["cd ?dirName?"],
    snippet: "Change the current working directory to dirName, or to the home directory (as specified in the HOME environment variable) if dirName is not given.",
    source: "Tcl man page cd.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
