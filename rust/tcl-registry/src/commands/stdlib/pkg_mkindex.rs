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
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Build a ``pkgIndex.tcl`` file for one or more packages.",
            &["pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?"],
            "F5",
        )),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
