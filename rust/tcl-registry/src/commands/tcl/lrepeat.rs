//! `lrepeat` — build a list by repeating elements.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrepeat",
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Build a list by repeating elements.",
            &["lrepeat count ?element ...?"],
            "Tcl lrepeat(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
