//! `snit::type` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::type",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Define a new snit type (class).",
            &["snit::type name definition"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
