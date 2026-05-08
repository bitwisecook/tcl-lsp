//! `TclOO` class.
use crate::prelude::*;

/// Resolve the body argument index for the metaclass shapes:
///
/// * `oo::class create Name body` → body at index 2.
/// * `oo::class new body` → body at index 1.
/// * `oo::class createWithNamespace Name ::ns body` → body at index 3.
fn oo_class_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n < 2 {
        return Vec::new();
    }
    match args[0] {
        "create" if n >= 3 => vec![(2, ArgRole::Body)],
        "new" if n >= 2 => vec![(1, ArgRole::Body)],
        "createWithNamespace" if n >= 4 => vec![(3, ArgRole::Body)],
        _ => Vec::new(),
    }
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::class",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // SYNC2: bodies of `oo::class create / new / createWithNamespace`
        // run in a TclOO definition context (not the caller's frame).
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Define or manipulate a `TclOO` class.",
            &["oo::class create name ?definition?"],
            "Tcl oo::class(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
