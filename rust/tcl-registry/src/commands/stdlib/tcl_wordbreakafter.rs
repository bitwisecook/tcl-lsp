//! `tcl_wordBreakAfter` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_wordBreakAfter",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the index of the first word boundary after *start* in *str*.",
            &["tcl_wordBreakAfter str start"],
            "F5",
        )),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
