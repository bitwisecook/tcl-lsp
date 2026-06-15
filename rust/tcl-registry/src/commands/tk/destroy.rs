//! `destroy` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "destroy ?window window ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "destroy",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Destroy one or more windows and all their descendants.",
            synopsis: &["destroy ?window window ...?"],
            snippet: "Destroys the specified windows and all of their descendants. If the main window (\".\") is destroyed the entire application is terminated.",
            source: "Tk man page destroy.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
