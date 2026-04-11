//! `time` — measure script execution time.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time",
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Time the execution of a script.",
            &["time script ?count?"],
            "Tcl time(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
