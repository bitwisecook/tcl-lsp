//! `tcltest::configure` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::configure",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set tcltest configuration options.",
            synopsis: &["tcltest::configure ?option? ?value option value ...?"],
            snippet: "Options include ``-verbose``, ``-debug``, ``-outfile``, ``-errfile``, ``-tmpdir``, ``-testdir``, ``-file``, ``-notfile``, ``-match``, ``-skip``, ``-constraints``, ``-limitconstraints``, ``-singleproc``, ``-preservecore``, ``-load``, ``-loadfile``.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
