//! `uri::register` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::register schemeList script",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::register",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        // SYNC2: `uri::register schemeList {script}` registers a
        // scheme handler — the script runs at parse time inside the
        // uri:: registration namespace, not the caller's scope.
        arg_roles: &[(1, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Register a new URI scheme handler.",
            synopsis: &["uri::register schemeList script"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}
