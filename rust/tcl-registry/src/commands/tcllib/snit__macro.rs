//! `snit::macro` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::macro name arglist body",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::macro",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Define a snit macro for use in type definitions.",
            synopsis: &["snit::macro name arglist body"],
            snippet: "",
            source: "tcllib snit package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(2, ArgRole::Body)],
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        ..CommandSpec::DEFAULT
    }
}
