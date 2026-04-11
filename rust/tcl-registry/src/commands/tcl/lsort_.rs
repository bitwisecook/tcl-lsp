//! `lsort` — sort a list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsort",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Sort the elements of a list.",
            &["lsort ?option ...? list"],
            "Tcl lsort(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
