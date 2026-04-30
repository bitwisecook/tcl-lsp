//! `regsub` helper aliases and regex quoting commands.
use crate::prelude::*;
/// Command spec for Tcl `re_quote` (regex quoting helper).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "re_quote",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Quote a string for use as a regex literal.",
            &["re_quote string"],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
