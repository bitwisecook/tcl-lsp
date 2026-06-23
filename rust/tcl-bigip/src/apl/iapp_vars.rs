//! Extract iApp presentation-variable references from implementation Tcl.
//!
//! In iApp templates a presentation field declared as `section.field` in
//! APL becomes `$::section__field` in the implementation Tcl. This module
//! extracts those references so they can be cross-validated against the
//! presentation model (see [`super::iapp_diagnostics`]).

use std::sync::LazyLock;

use regex::Regex;
use tcl_lexer::LineIndex;

use crate::range::Range;

/// Match `$::name` or `${::name}` (with an optional `(index)`). Like the
/// Python regex, the identifier is captured in one greedy chunk and the
/// iApp `__` separator is enforced by a post-filter rather than nested
/// quantifiers (which risk catastrophic backtracking — `CodeQL`
/// `py/redos`). Group 1 is the braced form, group 2 the unbraced form.
static IAPP_VAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\$\{::([a-zA-Z_][a-zA-Z0-9_]*)(?:\([^)]*\))?\}|\$::([a-zA-Z_][a-zA-Z0-9_]*)(?:\([^)]*\))?",
    )
    .expect("static iapp-var regex")
});

/// A reference to an iApp presentation variable in implementation Tcl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IappVarRef {
    /// The Tcl global name, e.g. `"::basic__addr"`.
    pub tcl_name: String,
    /// The APL qualified name, e.g. `"basic.addr"`.
    pub apl_name: String,
    /// Source range of the reference.
    pub range: Range,
}

/// Extract iApp presentation-variable references (`::section__field`)
/// from Tcl `source`. Plain `$::var` references without an `__` separator
/// are not iApp variables and are dropped.
#[must_use]
pub fn extract_iapp_var_refs(source: &str) -> Vec<IappVarRef> {
    let line_index = LineIndex::new(source);
    let mut refs: Vec<IappVarRef> = Vec::new();
    for caps in IAPP_VAR_RE.captures_iter(source) {
        // group 1 = braced, group 2 = unbraced.
        let Some(body) = caps.get(1).or_else(|| caps.get(2)) else {
            continue;
        };
        let var_body = body.as_str();
        // Real iApp refs contain at least one `__` separator.
        if !var_body.contains("__") {
            continue;
        }
        let whole = caps.get(0).expect("group 0 always present");
        refs.push(IappVarRef {
            tcl_name: format!("::{var_body}"),
            apl_name: var_body.replace("__", "."),
            range: Range::from_offsets(
                source,
                &line_index,
                whole.start(),
                whole.end().saturating_sub(1),
            ),
        });
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_braced_and_unbraced() {
        let src = "set a $::basic__addr\nset b ${::ssl__members__port}";
        let refs = extract_iapp_var_refs(src);
        let names: Vec<&str> = refs.iter().map(|r| r.apl_name.as_str()).collect();
        assert_eq!(names, vec!["basic.addr", "ssl.members.port"]);
        assert_eq!(refs[0].tcl_name, "::basic__addr");
    }

    #[test]
    fn drops_plain_non_iapp_vars() {
        // No `__` separator — not an iApp reference.
        assert!(extract_iapp_var_refs("set x $::tcl_version").is_empty());
    }

    #[test]
    fn ignores_array_index() {
        let refs = extract_iapp_var_refs("set v $::pool__members(0)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].apl_name, "pool.members");
    }
}
