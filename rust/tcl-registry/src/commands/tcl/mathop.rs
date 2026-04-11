//! `tcl::mathop` — namespace of math operator commands.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::mathop",
        traits: Traits::PURE,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Numeric),
        hover: Some(HoverSnippet::brief(
            "Mathematical operator commands.",
            &["tcl::mathop::op ?arg ...?"],
            "Tcl mathop(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
