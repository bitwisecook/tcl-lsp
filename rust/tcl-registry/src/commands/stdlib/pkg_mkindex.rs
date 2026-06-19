//! `pkg_mkIndex` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkIndex",
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Build a ``pkgIndex.tcl`` file for one or more packages.",
            synopsis: &[
                "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?",
            ],
            snippet: "Scans *dir* for Tcl source and binary files matching *pattern* (default ``*.tcl *.{so,dll}``) and builds a ``pkgIndex.tcl`` that enables ``package require`` to find them.",
            source: "Tcl stdlib package utilities",
            examples: "",
            return_value: "",
        }),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
