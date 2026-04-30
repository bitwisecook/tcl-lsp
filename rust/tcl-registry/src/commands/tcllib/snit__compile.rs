//! `snit::compile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::compile",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Compile a snit type definition into a Tcl script.",
            &["snit::compile which name body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
