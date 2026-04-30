//! `oo::define` — define class members.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::define",
        traits: Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define class members.",
            &[
                "oo::define className ?definition?",
                "oo::define className subcommand ?arg ...?",
            ],
            "Tcl oo::define(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
