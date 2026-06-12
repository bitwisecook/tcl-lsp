//! `auto_load_index` library proc.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load_index",
        // A redefinable Tcl library proc — see `Traits::OVERRIDABLE_LIBRARY_PROC`.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Rebuild the auto-load index from the auto_path directories",
            synopsis: &[],
            snippet: "",
            source: "Tcl library (init.tcl)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
