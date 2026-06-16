//! `pkg_mkindex` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkindex",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Build pkgIndex.tcl for a directory of packages",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
