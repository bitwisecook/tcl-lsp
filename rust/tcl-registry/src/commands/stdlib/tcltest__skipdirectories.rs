//! `tcltest::skipDirectories` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::skipDirectories",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary:
                "Get or set directory skip patterns.  Deprecated: use ``configure -asidefromdir``.",
            synopsis: &["tcltest::skipDirectories ?patternList?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
