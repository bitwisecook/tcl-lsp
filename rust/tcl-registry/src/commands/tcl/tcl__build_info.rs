//! `tcl::build-info` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::build-info",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Return compile-time build metadata for the Tcl runtime.",
            synopsis: &["tcl::build-info ?key?"],
            snippet: "Returns the compile-time build metadata for the running Tcl runtime.  With no arguments, returns the patchlevel.  With a key argument, returns the value associated with that key (e.g. ``version``, ``commit``, ``branch``, ``compiler``).",
            source: "Tcl tcl::build-info (internal)",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "tcl::build-info ?key?",
        }],
        ..CommandSpec::DEFAULT
    }
}
