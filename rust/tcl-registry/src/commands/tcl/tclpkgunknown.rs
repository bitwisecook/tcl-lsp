//! `tclPkgUnknown` library proc.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tclPkgUnknown",
        dialects: Some(DialectSet::ALL_TCL),
        // A redefinable Tcl library proc — see `Traits::OVERRIDABLE_LIBRARY_PROC`.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Default handler that loads pkgIndex.tcl files on demand",
            synopsis: &[],
            snippet: "",
            source: "Tcl library (init.tcl)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
