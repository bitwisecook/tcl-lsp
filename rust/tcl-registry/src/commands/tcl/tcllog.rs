//! `tclLog` library proc.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclLog",
        // A redefinable Tcl library proc — see `Traits::OVERRIDABLE_LIBRARY_PROC`.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Default logger for Tcl's library error messages",
            synopsis: &[],
            snippet: "",
            source: "Tcl library (init.tcl)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
