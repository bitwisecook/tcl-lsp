//! `snit::method` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::method",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        // SYNC2: snit method bodies run in a snit dispatch context,
        // not the caller's frame.  Body at index 3.
        arg_roles: &[(2, ArgRole::ParamList), (3, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Define an instance method outside a type definition body.",
            &["snit::method type name arglist body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
