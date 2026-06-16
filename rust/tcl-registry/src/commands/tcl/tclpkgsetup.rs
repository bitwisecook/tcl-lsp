//! `tclPkgSetup` library proc.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclPkgSetup",
        dialects: Some(DialectSet::ALL_TCL),
        // A redefinable Tcl library proc — see `Traits::OVERRIDABLE_LIBRARY_PROC`.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Set up auto-loading stubs for a package's commands",
            synopsis: &[],
            snippet: "",
            source: "Tcl library (init.tcl)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
