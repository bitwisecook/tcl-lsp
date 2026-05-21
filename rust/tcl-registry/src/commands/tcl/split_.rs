//! `split` — split a string into a list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "split",
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Split a string into a list.",
            &["split string ?splitChars?"],
            "Tcl split(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
