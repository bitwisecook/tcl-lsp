//! `lreverse` — reverse a list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreverse",
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Reverse a list.",
            &["lreverse list"],
            "Tcl lreverse(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
