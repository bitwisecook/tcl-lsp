//! `uri::register` command.
use crate::prelude::*;
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
        hover: Some(HoverSnippet::brief(
            "Register a new URI scheme handler.",
            &["uri::register schemeList script"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
